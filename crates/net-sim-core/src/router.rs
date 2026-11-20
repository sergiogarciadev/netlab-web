use std::collections::BTreeMap;
use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};

use crate::switch::MacAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArpEntry {
    pub mac: MacAddr,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterInterface {
    pub id: String,
    pub mac: MacAddr,
    pub ip: Option<Ipv4Addr>,
    pub prefix_len: u8,
    pub arp_cache: BTreeMap<Ipv4Addr, ArpEntry>,
    #[serde(skip)]
    pub pending_packets: BTreeMap<Ipv4Addr, Vec<Vec<u8>>>,
}

impl RouterInterface {
    pub fn new(id: impl Into<String>, mac: MacAddr) -> Self {
        Self {
            id: id.into(),
            mac,
            ip: None,
            prefix_len: 24,
            arp_cache: BTreeMap::new(),
            pending_packets: BTreeMap::new(),
        }
    }

    pub fn with_ip(mut self, ip: Ipv4Addr, prefix_len: u8) -> Self {
        self.ip = Some(ip);
        self.prefix_len = prefix_len;
        self
    }

    pub fn matches_subnet(&self, target: Ipv4Addr) -> bool {
        let Some(ip) = self.ip else { return false };
        ip_in_subnet(target, ip, self.prefix_len)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteEntry {
    pub network: Ipv4Addr,
    pub prefix_len: u8,
    pub next_hop: Option<Ipv4Addr>,
    pub out_iface: String,
}

/// Layer 3 IPv4 router with multi-interface forwarding, longest-prefix match, TTL decrement, and ARP.
#[derive(Debug, Clone)]
pub struct VirtualRouter {
    pub id: String,
    pub name: String,
    pub interfaces: BTreeMap<String, RouterInterface>,
    pub routes: Vec<RouteEntry>,
    pub arp_ttl_ms: u64,
}

impl VirtualRouter {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            interfaces: BTreeMap::new(),
            routes: Vec::new(),
            arp_ttl_ms: 120_000,
        }
    }

    pub fn add_interface(&mut self, iface: RouterInterface) {
        self.interfaces.insert(iface.id.clone(), iface);
    }

    pub fn configure_interface(
        &mut self,
        iface_id: &str,
        ip: Option<Ipv4Addr>,
        prefix_len: u8,
    ) -> Result<(), String> {
        let iface = self
            .interfaces
            .get_mut(iface_id)
            .ok_or_else(|| format!("Interface {iface_id} not found on router {}", self.id))?;
        iface.ip = ip;
        iface.prefix_len = prefix_len;
        Ok(())
    }

    pub fn add_route(&mut self, route: RouteEntry) {
        self.routes.push(route);
    }

    pub fn remove_route(&mut self, index: usize) {
        if index < self.routes.len() {
            self.routes.remove(index);
        }
    }

    /// Longest-prefix match across directly connected subnets and static routes.
    pub fn lookup_route(&self, dst: Ipv4Addr) -> Option<RouteEntry> {
        let mut best: Option<(u8, RouteEntry)> = None;

        // Connected subnets
        for iface in self.interfaces.values() {
            if let Some(ip) = iface.ip {
                if ip_in_subnet(dst, ip, iface.prefix_len) {
                    let net = mask_ip(ip, iface.prefix_len);
                    let candidate = RouteEntry {
                        network: net,
                        prefix_len: iface.prefix_len,
                        next_hop: None,
                        out_iface: iface.id.clone(),
                    };
                    if best
                        .as_ref()
                        .map_or(true, |(p, _)| iface.prefix_len >= *p)
                    {
                        best = Some((iface.prefix_len, candidate));
                    }
                }
            }
        }

        // Static routes
        for route in &self.routes {
            if ip_in_subnet(dst, route.network, route.prefix_len) {
                if best
                    .as_ref()
                    .map_or(true, |(p, _)| route.prefix_len > *p)
                {
                    best = Some((route.prefix_len, route.clone()));
                }
            }
        }

        best.map(|(_, r)| r)
    }

    /// Process an incoming Ethernet frame on `in_iface` and return `(out_iface, frame)` pairs.
    pub fn process_frame(
        &mut self,
        in_iface_id: &str,
        frame: Vec<u8>,
        now_ms: u64,
    ) -> Vec<(String, Vec<u8>)> {
        if frame.len() < 14 {
            return Vec::new();
        }
        let Some(iface) = self.interfaces.get(in_iface_id) else {
            return Vec::new();
        };

        let dst_mac: MacAddr = frame[0..6].try_into().unwrap();
        let is_broadcast = dst_mac == [0xff; 6];
        if dst_mac != iface.mac && !is_broadcast {
            return Vec::new();
        }

        let ethertype = u16::from_be_bytes([frame[12], frame[13]]);
        match ethertype {
            0x0806 => self.handle_arp(in_iface_id, &frame[14..], now_ms),
            0x0800 => self.handle_ipv4(in_iface_id, &frame[14..], now_ms),
            _ => Vec::new(),
        }
    }

    fn handle_arp(
        &mut self,
        in_iface_id: &str,
        payload: &[u8],
        now_ms: u64,
    ) -> Vec<(String, Vec<u8>)> {
        if payload.len() < 28 {
            return Vec::new();
        }
        let htype = u16::from_be_bytes([payload[0], payload[1]]);
        let ptype = u16::from_be_bytes([payload[2], payload[3]]);
        let hlen = payload[4];
        let plen = payload[5];
        if htype != 1 || ptype != 0x0800 || hlen != 6 || plen != 4 {
            return Vec::new();
        }

        let oper = u16::from_be_bytes([payload[6], payload[7]]);
        let sha: MacAddr = payload[8..14].try_into().unwrap();
        let spa = Ipv4Addr::new(payload[14], payload[15], payload[16], payload[17]);
        let tpa = Ipv4Addr::new(payload[24], payload[25], payload[26], payload[27]);

        let mut out = Vec::new();

        let (iface_mac, iface_ip) = {
            let Some(iface) = self.interfaces.get_mut(in_iface_id) else {
                return out;
            };
            iface.arp_cache.insert(
                spa,
                ArpEntry {
                    mac: sha,
                    expires_at_ms: now_ms + self.arp_ttl_ms,
                },
            );
            (iface.mac, iface.ip)
        };

        if oper == 1 {
            // ARP Request
            if iface_ip == Some(tpa) {
                let reply = build_arp_packet(2, iface_mac, tpa, sha, spa);
                let frame = wrap_ethernet(sha, iface_mac, 0x0806, &reply);
                out.push((in_iface_id.to_string(), frame));
            }
        }

        // Flush any pending packets waiting for `spa` on `in_iface_id`
        if let Some(iface) = self.interfaces.get_mut(in_iface_id) {
            if let Some(queued) = iface.pending_packets.remove(&spa) {
                for ip_pkt in queued {
                    let frame = wrap_ethernet(sha, iface.mac, 0x0800, &ip_pkt);
                    out.push((in_iface_id.to_string(), frame));
                }
            }
        }

        out
    }

    fn handle_ipv4(
        &mut self,
        in_iface_id: &str,
        ip_pkt: &[u8],
        now_ms: u64,
    ) -> Vec<(String, Vec<u8>)> {
        if ip_pkt.len() < 20 {
            return Vec::new();
        }
        let version = ip_pkt[0] >> 4;
        let ihl = ((ip_pkt[0] & 0x0f) as usize) * 4;
        if version != 4 || ihl < 20 || ip_pkt.len() < ihl {
            return Vec::new();
        }
        let total_len = u16::from_be_bytes([ip_pkt[2], ip_pkt[3]]) as usize;
        if total_len < ihl || total_len > ip_pkt.len() {
            return Vec::new();
        }
        let ip_slice = &ip_pkt[..total_len];

        let ttl = ip_slice[8];
        let proto = ip_slice[9];
        let src_ip = Ipv4Addr::new(ip_slice[12], ip_slice[13], ip_slice[14], ip_slice[15]);
        let dst_ip = Ipv4Addr::new(ip_slice[16], ip_slice[17], ip_slice[18], ip_slice[19]);

        // Check if addressed to one of this router's interfaces
        let is_for_me = self
            .interfaces
            .values()
            .any(|iface| iface.ip == Some(dst_ip));

        if is_for_me {
            // Respond to ICMP Echo Request
            if proto == 1 && total_len >= ihl + 8 && ip_slice[ihl] == 8 && ip_slice[ihl + 1] == 0 {
                let mut reply_ip = ip_slice.to_vec();
                // Swap src and dst IP
                reply_ip[12..16].copy_from_slice(&dst_ip.octets());
                reply_ip[16..20].copy_from_slice(&src_ip.octets());
                reply_ip[8] = 64; // Reset TTL
                // Set ICMP type = 0 (Echo Reply)
                reply_ip[ihl] = 0;
                // Recompute ICMP checksum
                reply_ip[ihl + 2] = 0;
                reply_ip[ihl + 3] = 0;
                let icmp_cksum = internet_checksum(&reply_ip[ihl..]);
                reply_ip[ihl + 2..ihl + 4].copy_from_slice(&icmp_cksum.to_be_bytes());
                // Recompute IPv4 header checksum
                reply_ip[10] = 0;
                reply_ip[11] = 0;
                let ip_cksum = internet_checksum(&reply_ip[..ihl]);
                reply_ip[10..12].copy_from_slice(&ip_cksum.to_be_bytes());

                return self.forward_ip_packet(reply_ip, src_ip, now_ms);
            }
            return Vec::new();
        }

        // Forwarding path: decrement TTL
        if ttl <= 1 {
            // Send ICMP Time Exceeded back to src_ip
            if let Some(in_ip) = self.interfaces.get(in_iface_id).and_then(|i| i.ip) {
                let icmp_err = build_icmp_error(in_ip, src_ip, 11, 0, ip_slice);
                return self.forward_ip_packet(icmp_err, src_ip, now_ms);
            }
            return Vec::new();
        }

        let mut fwd = ip_slice.to_vec();
        fwd[8] = ttl - 1;
        fwd[10] = 0;
        fwd[11] = 0;
        let cksum = internet_checksum(&fwd[..ihl]);
        fwd[10..12].copy_from_slice(&cksum.to_be_bytes());

        self.forward_ip_packet(fwd, dst_ip, now_ms)
    }

    fn forward_ip_packet(
        &mut self,
        ip_pkt: Vec<u8>,
        dst_ip: Ipv4Addr,
        now_ms: u64,
    ) -> Vec<(String, Vec<u8>)> {
        let Some(route) = self.lookup_route(dst_ip) else {
            return Vec::new();
        };

        let target_ip = route.next_hop.unwrap_or(dst_ip);
        let out_iface_id = route.out_iface;

        let Some(out_iface) = self.interfaces.get_mut(&out_iface_id) else {
            return Vec::new();
        };

        // Evict expired ARP entry if needed
        if let Some(entry) = out_iface.arp_cache.get(&target_ip) {
            if entry.expires_at_ms > now_ms {
                let frame = wrap_ethernet(entry.mac, out_iface.mac, 0x0800, &ip_pkt);
                return vec![(out_iface_id, frame)];
            } else {
                out_iface.arp_cache.remove(&target_ip);
            }
        }

        // Queue packet and emit ARP request
        let should_send_arp = !out_iface.pending_packets.contains_key(&target_ip);
        out_iface
            .pending_packets
            .entry(target_ip)
            .or_default()
            .push(ip_pkt);

        if should_send_arp {
            if let Some(src_ip) = out_iface.ip {
                let arp_req =
                    build_arp_packet(1, out_iface.mac, src_ip, [0x00; 6], target_ip);
                let frame = wrap_ethernet([0xff; 6], out_iface.mac, 0x0806, &arp_req);
                return vec![(out_iface_id, frame)];
            }
        }

        Vec::new()
    }
}

pub fn ip_in_subnet(ip: Ipv4Addr, net: Ipv4Addr, prefix_len: u8) -> bool {
    let mask = prefix_mask(prefix_len);
    (u32::from(ip) & mask) == (u32::from(net) & mask)
}

pub fn mask_ip(ip: Ipv4Addr, prefix_len: u8) -> Ipv4Addr {
    Ipv4Addr::from(u32::from(ip) & prefix_mask(prefix_len))
}

fn prefix_mask(prefix_len: u8) -> u32 {
    if prefix_len == 0 {
        0
    } else if prefix_len >= 32 {
        u32::MAX
    } else {
        u32::MAX << (32 - prefix_len)
    }
}

fn wrap_ethernet(dst: MacAddr, src: MacAddr, ethertype: u16, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(14 + payload.len());
    frame.extend_from_slice(&dst);
    frame.extend_from_slice(&src);
    frame.extend_from_slice(&ethertype.to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

fn build_arp_packet(
    oper: u16,
    sha: MacAddr,
    spa: Ipv4Addr,
    tha: MacAddr,
    tpa: Ipv4Addr,
) -> Vec<u8> {
    let mut pkt = Vec::with_capacity(28);
    pkt.extend_from_slice(&1u16.to_be_bytes()); // HTYPE Ethernet
    pkt.extend_from_slice(&0x0800u16.to_be_bytes()); // PTYPE IPv4
    pkt.push(6); // HLEN
    pkt.push(4); // PLEN
    pkt.extend_from_slice(&oper.to_be_bytes());
    pkt.extend_from_slice(&sha);
    pkt.extend_from_slice(&spa.octets());
    pkt.extend_from_slice(&tha);
    pkt.extend_from_slice(&tpa.octets());
    pkt
}

fn build_icmp_error(
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    icmp_type: u8,
    icmp_code: u8,
    orig_ip_pkt: &[u8],
) -> Vec<u8> {
    let ihl = ((orig_ip_pkt[0] & 0x0f) as usize) * 4;
    let copy_len = orig_ip_pkt.len().min(ihl + 8);
    let icmp_len = 8 + copy_len;
    let total_len = 20 + icmp_len;

    let mut pkt = vec![0u8; total_len];
    pkt[0] = 0x45;
    pkt[2..4].copy_from_slice(&(total_len as u16).to_be_bytes());
    pkt[8] = 64;
    pkt[9] = 1; // ICMP
    pkt[12..16].copy_from_slice(&src_ip.octets());
    pkt[16..20].copy_from_slice(&dst_ip.octets());

    pkt[20] = icmp_type;
    pkt[21] = icmp_code;
    pkt[28..28 + copy_len].copy_from_slice(&orig_ip_pkt[..copy_len]);

    let icmp_cksum = internet_checksum(&pkt[20..]);
    pkt[22..24].copy_from_slice(&icmp_cksum.to_be_bytes());

    let ip_cksum = internet_checksum(&pkt[..20]);
    pkt[10..12].copy_from_slice(&ip_cksum.to_be_bytes());
    pkt
}

pub fn internet_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut chunks = data.chunks_exact(2);
    for chunk in &mut chunks {
        sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
    }
    if let Some(&rem) = chunks.remainder().first() {
        sum += (rem as u32) << 8;
    }
    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}
