use std::collections::BTreeMap;
use std::net::Ipv4Addr;

use serde_json::{json, Value};
use smoltcp::iface::SocketHandle;
use smoltcp::socket::{dhcpv4, udp};
use smoltcp::wire::{IpAddress, IpCidr, IpEndpoint, Ipv4Address};

use crate::plugin::{
    AppPlugin, ConfigFieldSchema, PluginActionSchema, PluginContext, PluginMetadata,
};

pub struct DhcpClientPlugin {
    handle: Option<SocketHandle>,
    leased_ip: Option<String>,
    gateway: Option<String>,
    dns: Vec<String>,
}

impl DhcpClientPlugin {
    pub fn new() -> Self {
        Self {
            handle: None,
            leased_ip: None,
            gateway: None,
            dns: Vec::new(),
        }
    }

    pub fn metadata() -> PluginMetadata {
        PluginMetadata {
            id: "dhcp-client".into(),
            name: "DHCP Client".into(),
            description: "Automatically obtains IPv4 address, gateway, and DNS via DHCPv4.".into(),
            config_schema: vec![],
            actions: vec![PluginActionSchema {
                command: "renew".into(),
                label: "Renew Lease".into(),
                fields: vec![],
            }],
        }
    }
}

impl AppPlugin for DhcpClientPlugin {
    fn id(&self) -> &'static str {
        "dhcp-client"
    }

    fn name(&self) -> &'static str {
        "DHCP Client"
    }

    fn on_mount(&mut self, ctx: &mut PluginContext) {
        let sock = dhcpv4::Socket::new();
        self.handle = Some(ctx.sockets.add(sock));
    }

    fn on_unmount(&mut self, ctx: &mut PluginContext) {
        if let Some(h) = self.handle.take() {
            ctx.sockets.remove(h);
        }
    }

    fn poll(&mut self, ctx: &mut PluginContext) {
        let Some(h) = self.handle else { return };
        let sock = ctx.sockets.get_mut::<dhcpv4::Socket>(h);
        match sock.poll() {
            Some(dhcpv4::Event::Configured(config)) => {
                let ip = Ipv4Addr::from(config.address.address().octets());
                let prefix = config.address.prefix_len();
                *ctx.ip_cidr = Some((ip, prefix));
                ctx.iface.update_ip_addrs(|addrs| {
                    addrs.clear();
                    let _ = addrs.push(IpCidr::Ipv4(config.address));
                });

                ctx.iface.routes_mut().remove_default_ipv4_route();
                if let Some(router) = config.router {
                    let gw = Ipv4Addr::from(router.octets());
                    *ctx.gateway = Some(gw);
                    let _ = ctx.iface.routes_mut().add_default_ipv4_route(router);
                    self.gateway = Some(gw.to_string());
                } else {
                    *ctx.gateway = None;
                    self.gateway = None;
                }

                ctx.dns_servers.clear();
                self.dns.clear();
                for dns_ip in config.dns_servers.iter() {
                    let v4 = Ipv4Addr::from(dns_ip.octets());
                    ctx.dns_servers.push(v4);
                    self.dns.push(v4.to_string());
                }

                self.leased_ip = Some(format!("{ip}/{prefix}"));
                ctx.console_logs
                    .push(format!("[dhcp-client] Leased {ip}/{prefix}"));
            }
            Some(dhcpv4::Event::Deconfigured) => {
                *ctx.ip_cidr = None;
                *ctx.gateway = None;
                ctx.dns_servers.clear();
                self.leased_ip = None;
                self.gateway = None;
                self.dns.clear();
                ctx.iface.update_ip_addrs(|addrs| addrs.clear());
                ctx.iface.routes_mut().remove_default_ipv4_route();
            }
            None => {}
        }
    }

    fn handle_command(&mut self, cmd: Value, ctx: &mut PluginContext) -> Result<Value, String> {
        let action = cmd.get("command").and_then(|v| v.as_str()).unwrap_or("");
        if action == "configure" {
            return Ok(json!({ "ok": true }));
        }
        if action == "renew" {
            if let Some(h) = self.handle {
                ctx.sockets.get_mut::<dhcpv4::Socket>(h).reset();
            }
            return Ok(json!({ "status": "renewing" }));
        }
        Err(format!("Unknown dhcp-client command: {action}"))
    }

    fn get_state(&self) -> Value {
        json!({
            "leased_ip": self.leased_ip,
            "gateway": self.gateway,
            "dns_servers": self.dns,
        })
    }
}

pub struct DhcpServerPlugin {
    handle: Option<SocketHandle>,
    pool_start: Ipv4Addr,
    pool_end: Ipv4Addr,
    prefix_len: u8,
    gateway: Ipv4Addr,
    dns_server: Ipv4Addr,
    leases: BTreeMap<String, Ipv4Addr>,
}

impl DhcpServerPlugin {
    pub fn new() -> Self {
        Self {
            handle: None,
            pool_start: Ipv4Addr::new(10, 0, 1, 100),
            pool_end: Ipv4Addr::new(10, 0, 1, 200),
            prefix_len: 24,
            gateway: Ipv4Addr::new(10, 0, 1, 1),
            dns_server: Ipv4Addr::new(10, 0, 1, 53),
            leases: BTreeMap::new(),
        }
    }

    pub fn metadata() -> PluginMetadata {
        PluginMetadata {
            id: "dhcp-server".into(),
            name: "DHCP Server".into(),
            description: "Assigns IPv4 addresses, gateway, and DNS server to DHCP clients.".into(),
            config_schema: vec![
                ConfigFieldSchema {
                    key: "pool_start".into(),
                    label: "Pool Start IP".into(),
                    field_type: "text".into(),
                    default_value: json!("10.0.1.100"),
                },
                ConfigFieldSchema {
                    key: "pool_end".into(),
                    label: "Pool End IP".into(),
                    field_type: "text".into(),
                    default_value: json!("10.0.1.200"),
                },
                ConfigFieldSchema {
                    key: "prefix_len".into(),
                    label: "Prefix Length".into(),
                    field_type: "number".into(),
                    default_value: json!(24),
                },
                ConfigFieldSchema {
                    key: "gateway".into(),
                    label: "Default Gateway".into(),
                    field_type: "text".into(),
                    default_value: json!("10.0.1.1"),
                },
                ConfigFieldSchema {
                    key: "dns_server".into(),
                    label: "DNS Server IP".into(),
                    field_type: "text".into(),
                    default_value: json!("10.0.1.53"),
                },
            ],
            actions: vec![],
        }
    }

    fn allocate_ip(&mut self, mac: &str) -> Option<Ipv4Addr> {
        if let Some(&existing) = self.leases.get(mac) {
            return Some(existing);
        }
        let start = u32::from(self.pool_start);
        let end = u32::from(self.pool_end);
        for candidate in start..=end {
            let ip = Ipv4Addr::from(candidate);
            if !self.leases.values().any(|&v| v == ip) {
                self.leases.insert(mac.to_string(), ip);
                return Some(ip);
            }
        }
        None
    }
}

impl AppPlugin for DhcpServerPlugin {
    fn id(&self) -> &'static str {
        "dhcp-server"
    }

    fn name(&self) -> &'static str {
        "DHCP Server"
    }

    fn on_mount(&mut self, ctx: &mut PluginContext) {
        let rx_meta = vec![udp::PacketMetadata::EMPTY; 16];
        let tx_meta = vec![udp::PacketMetadata::EMPTY; 16];
        let mut sock = udp::Socket::new(
            udp::PacketBuffer::new(rx_meta, vec![0u8; 4096]),
            udp::PacketBuffer::new(tx_meta, vec![0u8; 4096]),
        );
        let _ = sock.bind(67);
        self.handle = Some(ctx.sockets.add(sock));
    }

    fn on_unmount(&mut self, ctx: &mut PluginContext) {
        if let Some(h) = self.handle.take() {
            ctx.sockets.remove(h);
        }
    }

    fn poll(&mut self, ctx: &mut PluginContext) {
        let Some(h) = self.handle else { return };
        let server_ip = ctx
            .ip_cidr
            .map(|(ip, _)| ip)
            .unwrap_or(Ipv4Addr::new(0, 0, 0, 0));
        if server_ip.is_unspecified() {
            return;
        }

        let mut pending_replies: Vec<Vec<u8>> = Vec::new();
        {
            let sock = ctx.sockets.get_mut::<udp::Socket>(h);
            while let Ok((data, _meta)) = sock.recv() {
                if data.len() < 244 || data[0] != 1 {
                    continue;
                }
                // Check magic cookie 99.130.83.99
                if data[236..240] != [99, 130, 83, 99] {
                    continue;
                }
                let xid = &data[4..8];
                let chaddr = &data[28..34];
                let mac_str = format!(
                    "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                    chaddr[0], chaddr[1], chaddr[2], chaddr[3], chaddr[4], chaddr[5]
                );

                // Parse option 53 (DHCP Message Type)
                let mut msg_type = 0u8;
                let mut i = 240;
                while i < data.len() {
                    let opt = data[i];
                    if opt == 255 {
                        break;
                    }
                    if opt == 0 {
                        i += 1;
                        continue;
                    }
                    if i + 1 >= data.len() {
                        break;
                    }
                    let len = data[i + 1] as usize;
                    if i + 2 + len > data.len() {
                        break;
                    }
                    if opt == 53 && len >= 1 {
                        msg_type = data[i + 2];
                    }
                    i += 2 + len;
                }

                let reply_type = match msg_type {
                    1 => 2, // DISCOVER -> OFFER
                    3 => 5, // REQUEST -> ACK
                    _ => continue,
                };

                if let Some(offered_ip) = self.allocate_ip(&mac_str) {
                    let pkt = build_dhcp_reply(
                        reply_type,
                        xid,
                        chaddr,
                        offered_ip,
                        server_ip,
                        self.prefix_len,
                        self.gateway,
                        self.dns_server,
                    );
                    pending_replies.push(pkt);
                    if reply_type == 5 {
                        ctx.console_logs
                            .push(format!("[dhcp-server] ACK {offered_ip} -> {mac_str}"));
                    }
                }
            }
        }

        if !pending_replies.is_empty() {
            let sock = ctx.sockets.get_mut::<udp::Socket>(h);
            let dst = IpEndpoint::new(IpAddress::Ipv4(Ipv4Address::BROADCAST), 68);
            for pkt in pending_replies {
                let _ = sock.send_slice(&pkt, dst);
            }
        }
    }

    fn handle_command(&mut self, cmd: Value, _ctx: &mut PluginContext) -> Result<Value, String> {
        let action = cmd.get("command").and_then(|v| v.as_str()).unwrap_or("");
        if action == "configure" {
            let cfg = cmd.get("config").unwrap_or(&cmd);
            if let Some(s) = cfg.get("pool_start").and_then(|v| v.as_str()) {
                self.pool_start = s.parse().map_err(|e| format!("Invalid pool_start: {e}"))?;
            }
            if let Some(s) = cfg.get("pool_end").and_then(|v| v.as_str()) {
                self.pool_end = s.parse().map_err(|e| format!("Invalid pool_end: {e}"))?;
            }
            if let Some(p) = cfg.get("prefix_len").and_then(|v| v.as_u64()) {
                self.prefix_len = p as u8;
            }
            if let Some(s) = cfg.get("gateway").and_then(|v| v.as_str()) {
                self.gateway = s.parse().map_err(|e| format!("Invalid gateway: {e}"))?;
            }
            if let Some(s) = cfg.get("dns_server").and_then(|v| v.as_str()) {
                self.dns_server = s.parse().map_err(|e| format!("Invalid dns_server: {e}"))?;
            }
            return Ok(json!({ "ok": true }));
        }
        Err(format!("Unknown dhcp-server command: {action}"))
    }

    fn get_state(&self) -> Value {
        let leases: BTreeMap<String, String> = self
            .leases
            .iter()
            .map(|(k, v)| (k.clone(), v.to_string()))
            .collect();
        json!({
            "pool_start": self.pool_start.to_string(),
            "pool_end": self.pool_end.to_string(),
            "prefix_len": self.prefix_len,
            "gateway": self.gateway.to_string(),
            "dns_server": self.dns_server.to_string(),
            "leases": leases,
        })
    }
}

fn build_dhcp_reply(
    msg_type: u8,
    xid: &[u8],
    chaddr: &[u8],
    yiaddr: Ipv4Addr,
    siaddr: Ipv4Addr,
    prefix_len: u8,
    gateway: Ipv4Addr,
    dns_server: Ipv4Addr,
) -> Vec<u8> {
    let mut pkt = vec![0u8; 300];
    pkt[0] = 2; // BOOTREPLY
    pkt[1] = 1; // Ethernet
    pkt[2] = 6; // HLEN
    pkt[4..8].copy_from_slice(xid);
    pkt[10] = 0x80; // Broadcast flag
    pkt[16..20].copy_from_slice(&yiaddr.octets());
    pkt[20..24].copy_from_slice(&siaddr.octets());
    pkt[28..34].copy_from_slice(chaddr);
    pkt[236..240].copy_from_slice(&[99, 130, 83, 99]); // Magic cookie

    let mut idx = 240;
    // Option 53: DHCP Message Type
    pkt[idx..idx + 3].copy_from_slice(&[53, 1, msg_type]);
    idx += 3;
    // Option 54: Server Identifier
    pkt[idx..idx + 2].copy_from_slice(&[54, 4]);
    pkt[idx + 2..idx + 6].copy_from_slice(&siaddr.octets());
    idx += 6;
    // Option 51: Lease Time (3600s)
    pkt[idx..idx + 2].copy_from_slice(&[51, 4]);
    pkt[idx + 2..idx + 6].copy_from_slice(&3600u32.to_be_bytes());
    idx += 6;
    // Option 1: Subnet Mask
    let mask = if prefix_len >= 32 {
        u32::MAX
    } else if prefix_len == 0 {
        0
    } else {
        u32::MAX << (32 - prefix_len)
    };
    pkt[idx..idx + 2].copy_from_slice(&[1, 4]);
    pkt[idx + 2..idx + 6].copy_from_slice(&mask.to_be_bytes());
    idx += 6;
    // Option 3: Router
    pkt[idx..idx + 2].copy_from_slice(&[3, 4]);
    pkt[idx + 2..idx + 6].copy_from_slice(&gateway.octets());
    idx += 6;
    // Option 6: DNS
    pkt[idx..idx + 2].copy_from_slice(&[6, 4]);
    pkt[idx + 2..idx + 6].copy_from_slice(&dns_server.octets());
    idx += 6;
    // End
    pkt[idx] = 255;
    pkt.truncate(idx + 1);
    pkt
}
