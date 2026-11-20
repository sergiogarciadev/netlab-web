use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub type MacAddr = [u8; 6];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacEntry {
    pub port_id: String,
    pub expires_at_ms: u64,
}

/// Layer 2 MAC-learning Ethernet switch.
#[derive(Debug, Clone)]
pub struct VirtualSwitch {
    pub id: String,
    pub name: String,
    pub ports: BTreeSet<String>,
    pub mac_table: BTreeMap<MacAddr, MacEntry>,
    pub mac_ttl_ms: u64,
}

impl VirtualSwitch {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            ports: BTreeSet::new(),
            mac_table: BTreeMap::new(),
            mac_ttl_ms: 300_000, // 5 minutes default aging
        }
    }

    pub fn add_port(&mut self, port_id: impl Into<String>) {
        self.ports.insert(port_id.into());
    }

    pub fn remove_port(&mut self, port_id: &str) {
        self.ports.remove(port_id);
        self.mac_table.retain(|_, entry| entry.port_id != port_id);
    }

    /// Evict expired MAC entries.
    pub fn expire_macs(&mut self, now_ms: u64) {
        self.mac_table.retain(|_, entry| entry.expires_at_ms > now_ms);
    }

    /// Process an incoming Ethernet frame on `in_port` and return `(out_port, frame)` pairs.
    pub fn process_frame(
        &mut self,
        in_port: &str,
        frame: Vec<u8>,
        now_ms: u64,
    ) -> Vec<(String, Vec<u8>)> {
        if frame.len() < 14 {
            return Vec::new();
        }

        self.expire_macs(now_ms);

        let dst_mac: MacAddr = frame[0..6].try_into().unwrap();
        let src_mac: MacAddr = frame[6..12].try_into().unwrap();

        // Learn source MAC if not multicast/broadcast
        if src_mac[0] & 0x01 == 0 {
            self.mac_table.insert(
                src_mac,
                MacEntry {
                    port_id: in_port.to_string(),
                    expires_at_ms: now_ms + self.mac_ttl_ms,
                },
            );
        }

        let is_broadcast_or_multicast = (dst_mac[0] & 0x01) != 0;

        if !is_broadcast_or_multicast {
            if let Some(entry) = self.mac_table.get(&dst_mac) {
                if entry.port_id == in_port {
                    // Hairpin filter: destination is on the same port it arrived on
                    return Vec::new();
                }
                return vec![(entry.port_id.clone(), frame)];
            }
        }

        // Flood to all other ports
        self.ports
            .iter()
            .filter(|p| p.as_str() != in_port)
            .map(|p| (p.clone(), frame.clone()))
            .collect()
    }
}
