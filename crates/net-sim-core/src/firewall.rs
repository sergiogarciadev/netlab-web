use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};

use crate::router::{ip_in_subnet, RouteEntry, RouterInterface, VirtualRouter};
use crate::switch::MacAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FirewallAction {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FirewallProtocol {
    Any,
    ICMP,
    TCP,
    UDP,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallRule {
    pub id: u64,
    pub action: FirewallAction,
    pub protocol: FirewallProtocol,
    pub src_cidr: Option<String>,
    pub dst_cidr: Option<String>,
    pub dst_port: Option<u16>,
    pub hits: u64,
}

/// Layer 3 stateful/packet-filtering firewall appliance.
#[derive(Debug, Clone)]
pub struct VirtualFirewall {
    pub id: String,
    pub name: String,
    pub router: VirtualRouter,
    pub rules: Vec<FirewallRule>,
    pub default_policy: FirewallAction,
    pub packets_allowed: u64,
    pub packets_dropped: u64,
    pub logs: Vec<String>,
    next_rule_id: u64,
}

impl VirtualFirewall {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        let id = id.into();
        let name = name.into();
        Self {
            router: VirtualRouter::new(id.clone(), name.clone()),
            id,
            name,
            rules: Vec::new(),
            default_policy: FirewallAction::Allow,
            packets_allowed: 0,
            packets_dropped: 0,
            logs: Vec::new(),
            next_rule_id: 1,
        }
    }

    pub fn add_interface(&mut self, iface: RouterInterface) {
        self.router.add_interface(iface);
    }

    pub fn configure_interface(
        &mut self,
        iface_id: &str,
        ip: Option<Ipv4Addr>,
        prefix_len: u8,
    ) -> Result<(), String> {
        self.router.configure_interface(iface_id, ip, prefix_len)
    }

    pub fn add_route(&mut self, route: RouteEntry) {
        self.router.add_route(route);
    }

    pub fn remove_route(&mut self, index: usize) {
        self.router.remove_route(index);
    }

    pub fn add_rule(
        &mut self,
        action: FirewallAction,
        protocol: FirewallProtocol,
        src_cidr: Option<String>,
        dst_cidr: Option<String>,
        dst_port: Option<u16>,
    ) -> u64 {
        let id = self.next_rule_id;
        self.next_rule_id += 1;
        self.rules.push(FirewallRule {
            id,
            action,
            protocol,
            src_cidr: src_cidr.filter(|s| !s.trim().is_empty()),
            dst_cidr: dst_cidr.filter(|s| !s.trim().is_empty()),
            dst_port,
            hits: 0,
        });
        id
    }

    pub fn remove_rule(&mut self, rule_id: u64) {
        self.rules.retain(|r| r.id != rule_id);
    }

    pub fn process_frame(
        &mut self,
        in_iface_id: &str,
        frame: Vec<u8>,
        now_ms: u64,
    ) -> Vec<(String, Vec<u8>)> {
        if frame.len() < 14 {
            return Vec::new();
        }
        let ethertype = u16::from_be_bytes([frame[12], frame[13]]);
        if ethertype != 0x0800 {
            // Always allow ARP and non-IPv4 control frames through to router stack
            return self.router.process_frame(in_iface_id, frame, now_ms);
        }

        let ip_pkt = &frame[14..];
        if ip_pkt.len() < 20 {
            return Vec::new();
        }
        let ihl = ((ip_pkt[0] & 0x0f) as usize) * 4;
        if ihl < 20 || ip_pkt.len() < ihl {
            return Vec::new();
        }

        let proto_num = ip_pkt[9];
        let src_ip = Ipv4Addr::new(ip_pkt[12], ip_pkt[13], ip_pkt[14], ip_pkt[15]);
        let dst_ip = Ipv4Addr::new(ip_pkt[16], ip_pkt[17], ip_pkt[18], ip_pkt[19]);

        let pkt_proto = match proto_num {
            1 => FirewallProtocol::ICMP,
            6 => FirewallProtocol::TCP,
            17 => FirewallProtocol::UDP,
            _ => FirewallProtocol::Any,
        };

        let dst_port = if (proto_num == 6 || proto_num == 17) && ip_pkt.len() >= ihl + 4 {
            Some(u16::from_be_bytes([ip_pkt[ihl + 2], ip_pkt[ihl + 3]]))
        } else {
            None
        };

        // Check if destination is addressed to the firewall's own interface
        let dst_mac: MacAddr = frame[0..6].try_into().unwrap();
        let Some(iface) = self.router.interfaces.get(in_iface_id) else {
            return Vec::new();
        };
        if dst_mac != iface.mac && dst_mac != [0xff; 6] {
            return Vec::new();
        }

        let mut matched_action: Option<(FirewallAction, String)> = None;
        for rule in &mut self.rules {
            if rule_matches(rule, pkt_proto, src_ip, dst_ip, dst_port) {
                rule.hits += 1;
                matched_action = Some((rule.action, format!("rule #{}", rule.id)));
                break;
            }
        }

        let (action, reason) = matched_action
            .unwrap_or((self.default_policy, "default policy".to_string()));

        match action {
            FirewallAction::Allow => {
                self.packets_allowed += 1;
                self.router.process_frame(in_iface_id, frame, now_ms)
            }
            FirewallAction::Deny => {
                self.packets_dropped += 1;
                let port_str = dst_port.map(|p| format!(":{p}")).unwrap_or_default();
                let log_line = format!(
                    "[{now_ms}ms] DROP {pkt_proto:?} {src_ip} -> {dst_ip}{port_str} ({reason})"
                );
                self.logs.push(log_line);
                if self.logs.len() > 100 {
                    self.logs.remove(0);
                }
                Vec::new()
            }
        }
    }
}

fn rule_matches(
    rule: &FirewallRule,
    pkt_proto: FirewallProtocol,
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    dst_port: Option<u16>,
) -> bool {
    if rule.protocol != FirewallProtocol::Any && rule.protocol != pkt_proto {
        return false;
    }
    if let Some(src_cidr) = &rule.src_cidr {
        if !matches_cidr(src_ip, src_cidr) {
            return false;
        }
    }
    if let Some(dst_cidr) = &rule.dst_cidr {
        if !matches_cidr(dst_ip, dst_cidr) {
            return false;
        }
    }
    if let Some(port) = rule.dst_port {
        if dst_port != Some(port) {
            return false;
        }
    }
    true
}

fn matches_cidr(ip: Ipv4Addr, cidr: &str) -> bool {
    let trimmed = cidr.trim();
    if trimmed.is_empty() || trimmed == "*" || trimmed.eq_ignore_ascii_case("any") {
        return true;
    }
    if let Some((net_str, prefix_str)) = trimmed.split_once('/') {
        let Ok(net) = net_str.parse::<Ipv4Addr>() else {
            return false;
        };
        let Ok(prefix) = prefix_str.parse::<u8>() else {
            return false;
        };
        ip_in_subnet(ip, net, prefix)
    } else if let Ok(exact) = trimmed.parse::<Ipv4Addr>() {
        ip == exact
    } else {
        false
    }
}
