use std::collections::BTreeMap;
use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use smoltcp::iface::SocketHandle;
use smoltcp::socket::udp;
use smoltcp::wire::{IpAddress, IpEndpoint, Ipv4Address};

use crate::plugin::{
    AppPlugin, ConfigFieldSchema, PluginActionSchema, PluginContext, PluginMetadata,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    pub record_type: String, // "A" or "CNAME"
    pub value: String,
}

pub struct DnsServerPlugin {
    handle: Option<SocketHandle>,
    records: BTreeMap<String, DnsRecord>,
    queries_served: u64,
}

impl DnsServerPlugin {
    pub fn new() -> Self {
        let mut records = BTreeMap::new();
        records.insert(
            "web.local".into(),
            DnsRecord {
                record_type: "A".into(),
                value: "10.0.2.10".into(),
            },
        );
        Self {
            handle: None,
            records,
            queries_served: 0,
        }
    }

    pub fn metadata() -> PluginMetadata {
        PluginMetadata {
            id: "dns-server".into(),
            name: "DNS Server".into(),
            description: "Authoritative DNS server supporting A and CNAME records on UDP/53."
                .into(),
            config_schema: vec![ConfigFieldSchema {
                key: "records_text".into(),
                label: "Zone Records (name TYPE value per line)".into(),
                field_type: "textarea".into(),
                default_value: json!("web.local A 10.0.2.10\nwww.web.local CNAME web.local"),
            }],
            actions: vec![PluginActionSchema {
                command: "add_record".into(),
                label: "Add Record".into(),
                fields: vec![
                    ConfigFieldSchema {
                        key: "name".into(),
                        label: "Hostname".into(),
                        field_type: "text".into(),
                        default_value: json!("mail.corp"),
                    },
                    ConfigFieldSchema {
                        key: "record_type".into(),
                        label: "Type (A or CNAME)".into(),
                        field_type: "text".into(),
                        default_value: json!("A"),
                    },
                    ConfigFieldSchema {
                        key: "value".into(),
                        label: "Value (IP or target)".into(),
                        field_type: "text".into(),
                        default_value: json!("10.0.2.25"),
                    },
                ],
            }],
        }
    }

    fn resolve_a(&self, name: &str) -> Option<Ipv4Addr> {
        let mut current = name.trim_end_matches('.').to_lowercase();
        for _ in 0..6 {
            let rec = self.records.get(&current)?;
            if rec.record_type.eq_ignore_ascii_case("A") {
                return rec.value.parse().ok();
            } else if rec.record_type.eq_ignore_ascii_case("CNAME") {
                current = rec.value.trim_end_matches('.').to_lowercase();
            } else {
                return None;
            }
        }
        None
    }
}

impl AppPlugin for DnsServerPlugin {
    fn id(&self) -> &'static str {
        "dns-server"
    }

    fn name(&self) -> &'static str {
        "DNS Server"
    }

    fn on_mount(&mut self, ctx: &mut PluginContext) {
        let rx_meta = vec![udp::PacketMetadata::EMPTY; 16];
        let tx_meta = vec![udp::PacketMetadata::EMPTY; 16];
        let mut sock = udp::Socket::new(
            udp::PacketBuffer::new(rx_meta, vec![0u8; 4096]),
            udp::PacketBuffer::new(tx_meta, vec![0u8; 4096]),
        );
        let _ = sock.bind(53);
        self.handle = Some(ctx.sockets.add(sock));
    }

    fn on_unmount(&mut self, ctx: &mut PluginContext) {
        if let Some(h) = self.handle.take() {
            ctx.sockets.remove(h);
        }
    }

    fn poll(&mut self, ctx: &mut PluginContext) {
        let Some(h) = self.handle else { return };
        let mut replies: Vec<(IpEndpoint, Vec<u8>)> = Vec::new();
        {
            let sock = ctx.sockets.get_mut::<udp::Socket>(h);
            while let Ok((data, meta)) = sock.recv() {
                if let Some((qname, q_end)) = parse_dns_question(data) {
                    let ip = self.resolve_a(&qname);
                    let resp = build_dns_response(data, q_end, ip);
                    self.queries_served += 1;
                    ctx.console_logs.push(format!(
                        "[dns-server] Query {} -> {}",
                        qname,
                        ip.map(|i| i.to_string()).unwrap_or_else(|| "NXDOMAIN".into())
                    ));
                    replies.push((meta.endpoint, resp));
                }
            }
        }
        if !replies.is_empty() {
            let sock = ctx.sockets.get_mut::<udp::Socket>(h);
            for (ep, pkt) in replies {
                let _ = sock.send_slice(&pkt, ep);
            }
        }
    }

    fn handle_command(&mut self, cmd: Value, _ctx: &mut PluginContext) -> Result<Value, String> {
        let action = cmd.get("command").and_then(|v| v.as_str()).unwrap_or("");
        match action {
            "configure" => {
                let cfg = cmd.get("config").unwrap_or(&cmd);
                if let Some(text) = cfg.get("records_text").and_then(|v| v.as_str()) {
                    self.records.clear();
                    for line in text.lines() {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 3 {
                            self.records.insert(
                                parts[0].to_lowercase(),
                                DnsRecord {
                                    record_type: parts[1].to_uppercase(),
                                    value: parts[2].to_string(),
                                },
                            );
                        }
                    }
                }
                Ok(json!({ "ok": true, "count": self.records.len() }))
            }
            "add_record" => {
                let name = cmd
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing name")?
                    .to_lowercase();
                let rtype = cmd
                    .get("record_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("A")
                    .to_uppercase();
                let value = cmd
                    .get("value")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing value")?
                    .to_string();
                self.records.insert(
                    name,
                    DnsRecord {
                        record_type: rtype,
                        value,
                    },
                );
                Ok(json!({ "ok": true }))
            }
            _ => Err(format!("Unknown dns-server command: {action}")),
        }
    }

    fn get_state(&self) -> Value {
        let records_text = self
            .records
            .iter()
            .map(|(k, v)| format!("{k} {} {}", v.record_type, v.value))
            .collect::<Vec<_>>()
            .join("\n");
        json!({
            "records": self.records,
            "records_text": records_text,
            "queries_served": self.queries_served,
        })
    }
}

pub struct DnsResolverPlugin {
    handle: Option<SocketHandle>,
    next_txid: u16,
    pending_queries: BTreeMap<u16, String>,
    last_results: Vec<String>,
}

impl DnsResolverPlugin {
    pub fn new() -> Self {
        Self {
            handle: None,
            next_txid: 1,
            pending_queries: BTreeMap::new(),
            last_results: Vec::new(),
        }
    }

    pub fn metadata() -> PluginMetadata {
        PluginMetadata {
            id: "dns-resolver".into(),
            name: "DNS Resolver".into(),
            description: "Resolves hostnames via UDP/53 against the configured DNS server."
                .into(),
            config_schema: vec![],
            actions: vec![PluginActionSchema {
                command: "lookup".into(),
                label: "Lookup Hostname".into(),
                fields: vec![ConfigFieldSchema {
                    key: "hostname".into(),
                    label: "Hostname".into(),
                    field_type: "text".into(),
                    default_value: json!("web.local"),
                }],
            }],
        }
    }
}

impl AppPlugin for DnsResolverPlugin {
    fn id(&self) -> &'static str {
        "dns-resolver"
    }

    fn name(&self) -> &'static str {
        "DNS Resolver"
    }

    fn on_mount(&mut self, ctx: &mut PluginContext) {
        let rx_meta = vec![udp::PacketMetadata::EMPTY; 16];
        let tx_meta = vec![udp::PacketMetadata::EMPTY; 16];
        let mut sock = udp::Socket::new(
            udp::PacketBuffer::new(rx_meta, vec![0u8; 4096]),
            udp::PacketBuffer::new(tx_meta, vec![0u8; 4096]),
        );
        let _ = sock.bind(53053);
        self.handle = Some(ctx.sockets.add(sock));
    }

    fn on_unmount(&mut self, ctx: &mut PluginContext) {
        if let Some(h) = self.handle.take() {
            ctx.sockets.remove(h);
        }
    }

    fn poll(&mut self, ctx: &mut PluginContext) {
        let Some(h) = self.handle else { return };
        let sock = ctx.sockets.get_mut::<udp::Socket>(h);
        while let Ok((data, _meta)) = sock.recv() {
            if let Some((txid, ip)) = parse_dns_a_answer(data) {
                if let Some(hostname) = self.pending_queries.remove(&txid) {
                    if let Some(addr) = ip {
                        ctx.dns_cache.insert(hostname.clone(), addr);
                        let msg = format!("{hostname} -> {addr}");
                        self.last_results.push(msg.clone());
                        ctx.console_logs.push(format!("[dns-resolver] {msg}"));
                    } else {
                        let msg = format!("{hostname} -> NXDOMAIN");
                        self.last_results.push(msg.clone());
                        ctx.console_logs.push(format!("[dns-resolver] {msg}"));
                    }
                }
            }
        }
    }

    fn handle_command(&mut self, cmd: Value, ctx: &mut PluginContext) -> Result<Value, String> {
        let action = cmd.get("command").and_then(|v| v.as_str()).unwrap_or("");
        if action == "configure" {
            return Ok(json!({ "ok": true }));
        }
        if action == "lookup" {
            let hostname = cmd
                .get("hostname")
                .and_then(|v| v.as_str())
                .ok_or("Missing hostname")?
                .trim()
                .to_lowercase();
            let dns_ip = if let Some(s) = cmd.get("server").and_then(|v| v.as_str()) {
                s.parse::<Ipv4Addr>()
                    .map_err(|e| format!("Invalid DNS server: {e}"))?
            } else {
                *ctx.dns_servers
                    .first()
                    .ok_or("No DNS server configured on host")?
            };

            let txid = self.next_txid;
            self.next_txid = self.next_txid.wrapping_add(1).max(1);
            let query = build_dns_query(txid, &hostname);
            let h = self.handle.ok_or("DNS resolver socket not mounted")?;
            let sock = ctx.sockets.get_mut::<udp::Socket>(h);
            let ep = IpEndpoint::new(IpAddress::Ipv4(Ipv4Address::from_octets(dns_ip.octets())), 53);
            sock.send_slice(&query, ep)
                .map_err(|e| format!("DNS send failed: {e:?}"))?;
            self.pending_queries.insert(txid, hostname.clone());
            return Ok(json!({ "status": "query_sent", "txid": txid, "hostname": hostname }));
        }
        Err(format!("Unknown dns-resolver command: {action}"))
    }

    fn get_state(&self) -> Value {
        json!({
            "results": self.last_results,
        })
    }
}

pub fn build_dns_query(txid: u16, hostname: &str) -> Vec<u8> {
    let mut pkt = Vec::with_capacity(64);
    pkt.extend_from_slice(&txid.to_be_bytes());
    pkt.extend_from_slice(&0x0100u16.to_be_bytes()); // Standard query, RD=1
    pkt.extend_from_slice(&1u16.to_be_bytes()); // QDCOUNT=1
    pkt.extend_from_slice(&0u16.to_be_bytes()); // ANCOUNT
    pkt.extend_from_slice(&0u16.to_be_bytes()); // NSCOUNT
    pkt.extend_from_slice(&0u16.to_be_bytes()); // ARCOUNT

    for label in hostname.trim_end_matches('.').split('.') {
        let bytes = label.as_bytes();
        pkt.push(bytes.len() as u8);
        pkt.extend_from_slice(bytes);
    }
    pkt.push(0); // Root
    pkt.extend_from_slice(&1u16.to_be_bytes()); // QTYPE = A
    pkt.extend_from_slice(&1u16.to_be_bytes()); // QCLASS = IN
    pkt
}

fn parse_dns_question(data: &[u8]) -> Option<(String, usize)> {
    if data.len() < 17 {
        return None;
    }
    let mut idx = 12;
    let mut labels = Vec::new();
    while idx < data.len() {
        let len = data[idx] as usize;
        idx += 1;
        if len == 0 {
            break;
        }
        if idx + len > data.len() {
            return None;
        }
        labels.push(String::from_utf8_lossy(&data[idx..idx + len]).to_string());
        idx += len;
    }
    if idx + 4 > data.len() {
        return None;
    }
    Some((labels.join("."), idx + 4))
}

fn build_dns_response(query: &[u8], q_end: usize, answer: Option<Ipv4Addr>) -> Vec<u8> {
    let mut resp = Vec::with_capacity(q_end + 16);
    resp.extend_from_slice(&query[..q_end]);
    if answer.is_some() {
        resp[2] = 0x81; // QR=1, RD=1
        resp[3] = 0x80; // RA=1, RCODE=0
        resp[6..8].copy_from_slice(&1u16.to_be_bytes()); // ANCOUNT=1
        // Name pointer to offset 12 (0xc00c)
        resp.extend_from_slice(&0xc00cu16.to_be_bytes());
        resp.extend_from_slice(&1u16.to_be_bytes()); // TYPE A
        resp.extend_from_slice(&1u16.to_be_bytes()); // CLASS IN
        resp.extend_from_slice(&60u32.to_be_bytes()); // TTL 60s
        resp.extend_from_slice(&4u16.to_be_bytes()); // RDLENGTH 4
        resp.extend_from_slice(&answer.unwrap().octets());
    } else {
        resp[2] = 0x81;
        resp[3] = 0x83; // NXDOMAIN
        resp[6..8].copy_from_slice(&0u16.to_be_bytes());
    }
    resp
}

fn parse_dns_a_answer(data: &[u8]) -> Option<(u16, Option<Ipv4Addr>)> {
    if data.len() < 12 {
        return None;
    }
    let txid = u16::from_be_bytes([data[0], data[1]]);
    let ancount = u16::from_be_bytes([data[6], data[7]]);
    if ancount == 0 || data.len() < 16 {
        return Some((txid, None));
    }
    // Last 4 bytes of a standard single-A-record response are the IPv4 address
    let len = data.len();
    let ip = Ipv4Addr::new(data[len - 4], data[len - 3], data[len - 2], data[len - 1]);
    Some((txid, Some(ip)))
}
