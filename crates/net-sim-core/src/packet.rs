use std::collections::VecDeque;
use std::net::Ipv4Addr;

use etherparse::{LinkHeader, NetHeaders, PacketHeaders, TransportHeader};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolLayer {
    pub name: String,
    pub fields: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketSummary {
    pub src_mac: String,
    pub dst_mac: String,
    pub ethertype: String,
    pub src_ip: Option<String>,
    pub dst_ip: Option<String>,
    pub protocol: String,
    pub src_port: Option<u16>,
    pub dst_port: Option<u16>,
    pub info: String,
    pub layers: Vec<ProtocolLayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedPacket {
    pub seq: u64,
    pub timestamp_ms: u64,
    pub cable_id: String,
    pub from_node: String,
    pub from_port: String,
    pub to_node: String,
    pub to_port: String,
    pub length: usize,
    pub raw: Vec<u8>,
    pub summary: PacketSummary,
}

pub struct PacketTap {
    pub max_packets: usize,
    pub history: VecDeque<CapturedPacket>,
    pending_batch: Vec<CapturedPacket>,
    next_seq: u64,
}

impl Default for PacketTap {
    fn default() -> Self {
        Self::new(2000)
    }
}

impl PacketTap {
    pub fn new(max_packets: usize) -> Self {
        Self {
            max_packets,
            history: VecDeque::with_capacity(max_packets.min(256)),
            pending_batch: Vec::new(),
            next_seq: 1,
        }
    }

    pub fn record(
        &mut self,
        timestamp_ms: u64,
        cable_id: impl Into<String>,
        from_node: impl Into<String>,
        from_port: impl Into<String>,
        to_node: impl Into<String>,
        to_port: impl Into<String>,
        raw: &[u8],
    ) {
        let seq = self.next_seq;
        self.next_seq += 1;
        let summary = dissect_ethernet_frame(raw);
        let pkt = CapturedPacket {
            seq,
            timestamp_ms,
            cable_id: cable_id.into(),
            from_node: from_node.into(),
            from_port: from_port.into(),
            to_node: to_node.into(),
            to_port: to_port.into(),
            length: raw.len(),
            raw: raw.to_vec(),
            summary,
        };

        if self.history.len() >= self.max_packets {
            self.history.pop_front();
        }
        self.history.push_back(pkt.clone());
        self.pending_batch.push(pkt);
    }

    pub fn drain_batch(&mut self) -> Vec<CapturedPacket> {
        std::mem::take(&mut self.pending_batch)
    }

    pub fn clear(&mut self) {
        self.history.clear();
        self.pending_batch.clear();
    }

    /// Generate a standard Libpcap binary file (LINKTYPE_ETHERNET = 1).
    pub fn export_pcap(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(24 + self.history.len() * 96);
        // Global header (24 bytes, little-endian)
        out.extend_from_slice(&0xa1b2c3d4u32.to_le_bytes()); // magic_number
        out.extend_from_slice(&2u16.to_le_bytes()); // version_major
        out.extend_from_slice(&4u16.to_le_bytes()); // version_minor
        out.extend_from_slice(&0i32.to_le_bytes()); // thiszone
        out.extend_from_slice(&0u32.to_le_bytes()); // sigfigs
        out.extend_from_slice(&65535u32.to_le_bytes()); // snaplen
        out.extend_from_slice(&1u32.to_le_bytes()); // network (LINKTYPE_ETHERNET)

        for pkt in &self.history {
            let ts_sec = (pkt.timestamp_ms / 1000) as u32;
            let ts_usec = ((pkt.timestamp_ms % 1000) * 1000) as u32;
            let len = pkt.raw.len() as u32;
            out.extend_from_slice(&ts_sec.to_le_bytes());
            out.extend_from_slice(&ts_usec.to_le_bytes());
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(&pkt.raw);
        }
        out
    }
}

fn fmt_mac(m: &[u8; 6]) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        m[0], m[1], m[2], m[3], m[4], m[5]
    )
}

pub fn dissect_ethernet_frame(raw: &[u8]) -> PacketSummary {
    let mut layers = Vec::new();
    let mut src_mac = String::from("00:00:00:00:00:00");
    let mut dst_mac = String::from("00:00:00:00:00:00");
    let mut ethertype = String::from("0x0000");
    let mut src_ip: Option<String> = None;
    let mut dst_ip: Option<String> = None;
    let mut protocol = String::from("ETH");
    let mut src_port: Option<u16> = None;
    let mut dst_port: Option<u16> = None;
    let mut info = format!("Ethernet Frame ({} bytes)", raw.len());

    let Ok(parsed) = PacketHeaders::from_ethernet_slice(raw) else {
        return PacketSummary {
            src_mac,
            dst_mac,
            ethertype,
            src_ip,
            dst_ip,
            protocol,
            src_port,
            dst_port,
            info,
            layers,
        };
    };

    if let Some(LinkHeader::Ethernet2(eth)) = &parsed.link {
        src_mac = fmt_mac(&eth.source);
        dst_mac = fmt_mac(&eth.destination);
        ethertype = format!("0x{:04x}", u16::from(eth.ether_type));
        layers.push(ProtocolLayer {
            name: format!("Ethernet II, Src: {src_mac}, Dst: {dst_mac}"),
            fields: vec![
                ("Destination".into(), dst_mac.clone()),
                ("Source".into(), src_mac.clone()),
                ("Type".into(), ethertype.clone()),
            ],
        });
    }

    match &parsed.net {
        Some(NetHeaders::Arp(arp)) => {
            protocol = "ARP".into();
            let op_str = match arp.operation.0 {
                1 => "Request",
                2 => "Reply",
                _ => "Other",
            };
            let spa_bytes = arp.sender_protocol_addr();
            let spa = if spa_bytes.len() == 4 {
                Ipv4Addr::new(spa_bytes[0], spa_bytes[1], spa_bytes[2], spa_bytes[3]).to_string()
            } else {
                String::new()
            };
            let tpa_bytes = arp.target_protocol_addr();
            let tpa = if tpa_bytes.len() == 4 {
                Ipv4Addr::new(tpa_bytes[0], tpa_bytes[1], tpa_bytes[2], tpa_bytes[3]).to_string()
            } else {
                String::new()
            };
            src_ip = Some(spa.clone());
            dst_ip = Some(tpa.clone());
            info = if arp.operation.0 == 1 {
                format!("Who has {tpa}? Tell {spa}")
            } else {
                format!("{spa} is at {src_mac}")
            };
            layers.push(ProtocolLayer {
                name: format!("Address Resolution Protocol ({op_str})"),
                fields: vec![
                    ("Opcode".into(), format!("{} ({op_str})", arp.operation.0)),
                    ("Sender IP".into(), spa),
                    ("Target IP".into(), tpa),
                ],
            });
        }
        Some(NetHeaders::Ipv4(ip4, _)) => {
            protocol = "IPv4".into();
            let s = Ipv4Addr::from(ip4.source).to_string();
            let d = Ipv4Addr::from(ip4.destination).to_string();
            src_ip = Some(s.clone());
            dst_ip = Some(d.clone());
            layers.push(ProtocolLayer {
                name: format!("Internet Protocol Version 4, Src: {s}, Dst: {d}"),
                fields: vec![
                    ("Source Address".into(), s),
                    ("Destination Address".into(), d),
                    ("Time to Live".into(), ip4.time_to_live.to_string()),
                    ("Protocol".into(), format!("{:?}", ip4.protocol)),
                    ("Total Length".into(), ip4.total_len.to_string()),
                ],
            });
        }
        Some(NetHeaders::Ipv6(ip6, _)) => {
            protocol = "IPv6".into();
            let s = std::net::Ipv6Addr::from(ip6.source).to_string();
            let d = std::net::Ipv6Addr::from(ip6.destination).to_string();
            src_ip = Some(s.clone());
            dst_ip = Some(d.clone());
            layers.push(ProtocolLayer {
                name: format!("Internet Protocol Version 6, Src: {s}, Dst: {d}"),
                fields: vec![
                    ("Source Address".into(), s),
                    ("Destination Address".into(), d),
                    ("Hop Limit".into(), ip6.hop_limit.to_string()),
                ],
            });
        }
        _ => {}
    }

    let payload_slice = parsed.payload.slice();

    match &parsed.transport {
        Some(TransportHeader::Icmpv4(icmp)) => {
            protocol = "ICMP".into();
            let desc = match &icmp.icmp_type {
                etherparse::Icmpv4Type::EchoRequest(h) => {
                    format!("Echo (ping) request id=0x{:04x}, seq={}", h.id, h.seq)
                }
                etherparse::Icmpv4Type::EchoReply(h) => {
                    format!("Echo (ping) reply id=0x{:04x}, seq={}", h.id, h.seq)
                }
                etherparse::Icmpv4Type::TimeExceeded(_) => "Time-to-live exceeded".into(),
                etherparse::Icmpv4Type::DestinationUnreachable(_) => "Destination unreachable".into(),
                other => format!("{other:?}"),
            };
            info = desc.clone();
            layers.push(ProtocolLayer {
                name: "Internet Control Message Protocol".into(),
                fields: vec![
                    ("Type".into(), desc),
                    ("Checksum".into(), format!("0x{:04x}", icmp.checksum)),
                ],
            });
        }
        Some(TransportHeader::Icmpv6(icmp6)) => {
            protocol = "ICMPv6".into();
            info = format!("{:?}", icmp6.icmp_type);
            layers.push(ProtocolLayer {
                name: "Internet Control Message Protocol v6".into(),
                fields: vec![("Type".into(), info.clone())],
            });
        }
        Some(TransportHeader::Tcp(tcp)) => {
            protocol = "TCP".into();
            src_port = Some(tcp.source_port);
            dst_port = Some(tcp.destination_port);
            let mut flags = Vec::new();
            if tcp.syn { flags.push("SYN"); }
            if tcp.ack { flags.push("ACK"); }
            if tcp.fin { flags.push("FIN"); }
            if tcp.rst { flags.push("RST"); }
            if tcp.psh { flags.push("PSH"); }
            let flag_str = if flags.is_empty() {
                "NONE".into()
            } else {
                flags.join(", ")
            };
            info = format!(
                "{} -> {} [{}] Seq={} Ack={} Win={} Len={}",
                tcp.source_port,
                tcp.destination_port,
                flag_str,
                tcp.sequence_number,
                tcp.acknowledgment_number,
                tcp.window_size,
                payload_slice.len()
            );
            layers.push(ProtocolLayer {
                name: format!(
                    "Transmission Control Protocol, Src Port: {}, Dst Port: {}",
                    tcp.source_port, tcp.destination_port
                ),
                fields: vec![
                    ("Source Port".into(), tcp.source_port.to_string()),
                    ("Destination Port".into(), tcp.destination_port.to_string()),
                    ("Sequence Number".into(), tcp.sequence_number.to_string()),
                    ("Acknowledgment Number".into(), tcp.acknowledgment_number.to_string()),
                    ("Flags".into(), flag_str),
                    ("Window".into(), tcp.window_size.to_string()),
                ],
            });
            if !payload_slice.is_empty() && (tcp.source_port == 80 || tcp.destination_port == 80 || tcp.source_port == 8080 || tcp.destination_port == 8080) {
                let text = String::from_utf8_lossy(&payload_slice[..payload_slice.len().min(120)]);
                if text.starts_with("GET ") || text.starts_with("POST ") || text.starts_with("HTTP/") {
                    protocol = "HTTP".into();
                    if let Some(first_line) = text.lines().next() {
                        info = first_line.to_string();
                    }
                }
            }
        }
        Some(TransportHeader::Udp(udp)) => {
            protocol = "UDP".into();
            src_port = Some(udp.source_port);
            dst_port = Some(udp.destination_port);
            info = format!(
                "{} -> {} Len={}",
                udp.source_port,
                udp.destination_port,
                payload_slice.len()
            );
            if udp.source_port == 53 || udp.destination_port == 53 {
                protocol = "DNS".into();
                info = format!("DNS {} -> {} ({} bytes)", udp.source_port, udp.destination_port, payload_slice.len());
            } else if (udp.source_port == 67 && udp.destination_port == 68)
                || (udp.source_port == 68 && udp.destination_port == 67)
            {
                protocol = "DHCP".into();
                info = format!("DHCP {} -> {} ({} bytes)", udp.source_port, udp.destination_port, payload_slice.len());
            }
            layers.push(ProtocolLayer {
                name: format!(
                    "User Datagram Protocol, Src Port: {}, Dst Port: {}",
                    udp.source_port, udp.destination_port
                ),
                fields: vec![
                    ("Source Port".into(), udp.source_port.to_string()),
                    ("Destination Port".into(), udp.destination_port.to_string()),
                    ("Length".into(), udp.length.to_string()),
                ],
            });
        }
        _ => {}
    }

    if !payload_slice.is_empty() {
        let preview = String::from_utf8_lossy(&payload_slice[..payload_slice.len().min(80)]);
        layers.push(ProtocolLayer {
            name: format!("Application Payload ({} bytes)", payload_slice.len()),
            fields: vec![("Preview".into(), preview.to_string())],
        });
    }

    PacketSummary {
        src_mac,
        dst_mac,
        ethertype,
        src_ip,
        dst_ip,
        protocol,
        src_port,
        dst_port,
        info,
        layers,
    }
}
