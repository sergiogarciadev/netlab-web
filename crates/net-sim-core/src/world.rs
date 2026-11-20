use std::collections::{BTreeMap, HashMap};
use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};

use crate::cable::{CableConfig, CableDirection, Endpoint, VirtualCable};
use crate::host::VirtualHost;
use crate::packet::{CapturedPacket, PacketTap};
use crate::router::{RouteEntry, RouterInterface, VirtualRouter};
use crate::switch::{MacAddr, VirtualSwitch};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    Host,
    Switch,
    Router,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeAnimationEvent {
    pub cable_id: String,
    pub direction: CableDirection,
    pub start_ms: u64,
    pub deliver_at_ms: u64,
    pub protocol: String,
    pub size: usize,
}

/// Top-level deterministic network simulation world.
pub struct NetworkWorld {
    pub now_ms: u64,
    pub hosts: BTreeMap<String, VirtualHost>,
    pub switches: BTreeMap<String, VirtualSwitch>,
    pub routers: BTreeMap<String, VirtualRouter>,
    pub cables: BTreeMap<String, VirtualCable>,
    /// Maps (node_id, port_id) -> cable_id for O(1) lookup when transmitting.
    port_to_cable: HashMap<(String, String), String>,
    next_mac_seq: u64,
    next_seed: u64,
    pub animation_events: Vec<EdgeAnimationEvent>,
    pub packet_tap: PacketTap,
}

impl Default for NetworkWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkWorld {
    pub fn new() -> Self {
        Self {
            now_ms: 0,
            hosts: BTreeMap::new(),
            switches: BTreeMap::new(),
            routers: BTreeMap::new(),
            cables: BTreeMap::new(),
            port_to_cable: HashMap::new(),
            next_mac_seq: 1,
            next_seed: 0xdead_beef_cafe_babe,
            animation_events: Vec::new(),
            packet_tap: PacketTap::default(),
        }
    }

    fn alloc_mac(&mut self) -> MacAddr {
        let seq = self.next_mac_seq;
        self.next_mac_seq += 1;
        let bytes = seq.to_be_bytes();
        // Locally administered unicast MAC: 02:00:xx:xx:xx:xx
        [0x02, 0x00, bytes[4], bytes[5], bytes[6], bytes[7]]
    }

    fn alloc_seed(&mut self) -> u64 {
        self.next_seed = self.next_seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        self.next_seed
    }

    pub fn add_host(&mut self, id: impl Into<String>, name: impl Into<String>) -> &mut VirtualHost {
        let id = id.into();
        let mac = self.alloc_mac();
        let host = VirtualHost::new(id.clone(), name, mac, self.now_ms);
        self.hosts.insert(id.clone(), host);
        self.hosts.get_mut(&id).unwrap()
    }

    pub fn add_switch(
        &mut self,
        id: impl Into<String>,
        name: impl Into<String>,
    ) -> &mut VirtualSwitch {
        let id = id.into();
        let sw = VirtualSwitch::new(id.clone(), name);
        self.switches.insert(id.clone(), sw);
        self.switches.get_mut(&id).unwrap()
    }

    pub fn add_router(
        &mut self,
        id: impl Into<String>,
        name: impl Into<String>,
    ) -> &mut VirtualRouter {
        let id = id.into();
        let router = VirtualRouter::new(id.clone(), name);
        self.routers.insert(id.clone(), router);
        self.routers.get_mut(&id).unwrap()
    }

    pub fn remove_node(&mut self, node_id: &str) {
        self.hosts.remove(node_id);
        self.switches.remove(node_id);
        self.routers.remove(node_id);

        let attached_cables: Vec<String> = self
            .cables
            .iter()
            .filter(|(_, c)| c.end_a.node_id == node_id || c.end_b.node_id == node_id)
            .map(|(id, _)| id.clone())
            .collect();

        for cable_id in attached_cables {
            self.disconnect(&cable_id);
        }
    }

    pub fn connect(
        &mut self,
        cable_id: impl Into<String>,
        end_a: Endpoint,
        end_b: Endpoint,
    ) -> Result<(), String> {
        let cable_id = cable_id.into();
        if self.cables.contains_key(&cable_id) {
            return Err(format!("Cable {cable_id} already exists"));
        }
        let key_a = (end_a.node_id.clone(), end_a.port_id.clone());
        let key_b = (end_b.node_id.clone(), end_b.port_id.clone());
        if self.port_to_cable.contains_key(&key_a) {
            return Err(format!("Port {}:{} is already connected", end_a.node_id, end_a.port_id));
        }
        if self.port_to_cable.contains_key(&key_b) {
            return Err(format!("Port {}:{} is already connected", end_b.node_id, end_b.port_id));
        }

        // Ensure switch ports / router interfaces exist
        self.ensure_port_exists(&end_a);
        self.ensure_port_exists(&end_b);

        let seed = self.alloc_seed();
        let cable = VirtualCable::new(cable_id.clone(), end_a, end_b, seed);
        self.cables.insert(cable_id.clone(), cable);
        self.port_to_cable.insert(key_a, cable_id.clone());
        self.port_to_cable.insert(key_b, cable_id);
        Ok(())
    }

    fn ensure_port_exists(&mut self, ep: &Endpoint) {
        if let Some(sw) = self.switches.get_mut(&ep.node_id) {
            sw.add_port(ep.port_id.clone());
        } else if self.routers.contains_key(&ep.node_id) {
            let needs_iface = self
                .routers
                .get(&ep.node_id)
                .map_or(false, |r| !r.interfaces.contains_key(&ep.port_id));
            if needs_iface {
                let mac = self.alloc_mac();
                if let Some(r) = self.routers.get_mut(&ep.node_id) {
                    r.add_interface(RouterInterface::new(ep.port_id.clone(), mac));
                }
            }
        }
    }

    pub fn disconnect(&mut self, cable_id: &str) {
        if let Some(cable) = self.cables.remove(cable_id) {
            self.port_to_cable
                .remove(&(cable.end_a.node_id.clone(), cable.end_a.port_id.clone()));
            self.port_to_cable
                .remove(&(cable.end_b.node_id.clone(), cable.end_b.port_id.clone()));

            if let Some(sw) = self.switches.get_mut(&cable.end_a.node_id) {
                sw.remove_port(&cable.end_a.port_id);
            }
            if let Some(sw) = self.switches.get_mut(&cable.end_b.node_id) {
                sw.remove_port(&cable.end_b.port_id);
            }
        }
    }

    pub fn update_cable_config(&mut self, cable_id: &str, config: CableConfig) -> Result<(), String> {
        let cable = self
            .cables
            .get_mut(cable_id)
            .ok_or_else(|| format!("Cable {cable_id} not found"))?;
        cable.config = config;
        Ok(())
    }

    pub fn configure_host_ip(
        &mut self,
        host_id: &str,
        ip: Option<Ipv4Addr>,
        prefix_len: u8,
        gateway: Option<Ipv4Addr>,
    ) -> Result<(), String> {
        let host = self
            .hosts
            .get_mut(host_id)
            .ok_or_else(|| format!("Host {host_id} not found"))?;
        host.configure_ip(ip, prefix_len, gateway);
        Ok(())
    }

    pub fn configure_router_interface(
        &mut self,
        router_id: &str,
        iface_id: &str,
        ip: Option<Ipv4Addr>,
        prefix_len: u8,
    ) -> Result<(), String> {
        if !self.routers.contains_key(router_id) {
            return Err(format!("Router {router_id} not found"));
        }
        let needs_iface = self
            .routers
            .get(router_id)
            .map_or(false, |r| !r.interfaces.contains_key(iface_id));
        if needs_iface {
            let mac = self.alloc_mac();
            if let Some(r) = self.routers.get_mut(router_id) {
                r.add_interface(RouterInterface::new(iface_id, mac));
            }
        }
        self.routers
            .get_mut(router_id)
            .unwrap()
            .configure_interface(iface_id, ip, prefix_len)
    }

    pub fn add_router_route(&mut self, router_id: &str, route: RouteEntry) -> Result<(), String> {
        let router = self
            .routers
            .get_mut(router_id)
            .ok_or_else(|| format!("Router {router_id} not found"))?;
        router.add_route(route);
        Ok(())
    }

    pub fn send_ping(
        &mut self,
        host_id: &str,
        target: Ipv4Addr,
        ident: u16,
        seq: u16,
    ) -> Result<(), String> {
        let now_ms = self.now_ms;
        let host = self
            .hosts
            .get_mut(host_id)
            .ok_or_else(|| format!("Host {host_id} not found"))?;
        host.send_ping(target, ident, seq, now_ms)
    }

    /// Advance the simulation deterministically by `delta_ms` milliseconds in 1ms ticks.
    pub fn step(&mut self, delta_ms: u64) {
        for _ in 0..delta_ms {
            self.tick_one_ms();
            self.now_ms += 1;
        }
    }

    fn tick_one_ms(&mut self) {
        let now_ms = self.now_ms;

        // 1. Collect all due frames from all cables
        let mut deliveries: Vec<(Endpoint, Vec<u8>)> = Vec::new();
        for cable in self.cables.values_mut() {
            deliveries.extend(cable.pop_due(now_ms));
        }

        // 2. Deliver frames to hosts, switches, and routers
        let mut outbound: Vec<(Endpoint, Vec<u8>)> = Vec::new();

        for (target, frame) in deliveries {
            if let Some(host) = self.hosts.get_mut(&target.node_id) {
                host.device.push_rx(frame);
            } else if let Some(sw) = self.switches.get_mut(&target.node_id) {
                let forwarded = sw.process_frame(&target.port_id, frame, now_ms);
                for (out_port, out_frame) in forwarded {
                    outbound.push((Endpoint::new(target.node_id.clone(), out_port), out_frame));
                }
            } else if let Some(router) = self.routers.get_mut(&target.node_id) {
                let forwarded = router.process_frame(&target.port_id, frame, now_ms);
                for (out_iface, out_frame) in forwarded {
                    outbound.push((Endpoint::new(target.node_id.clone(), out_iface), out_frame));
                }
            }
        }

        // 3. Poll all hosts and drain their TX queues
        for (host_id, host) in self.hosts.iter_mut() {
            host.poll(now_ms);
            while let Some(frame) = host.device.pop_tx() {
                outbound.push((Endpoint::new(host_id.clone(), "eth0"), frame));
            }
        }

        // 4. Inject all outbound frames into their connected cables
        for (from_ep, frame) in outbound {
            let key = (from_ep.node_id.clone(), from_ep.port_id.clone());
            if let Some(cable_id) = self.port_to_cable.get(&key) {
                if let Some(cable) = self.cables.get_mut(cable_id) {
                    let size = frame.len();
                    let protocol = classify_frame_protocol(&frame);
                    let to_ep = if cable.end_a.node_id == from_ep.node_id
                        && cable.end_a.port_id == from_ep.port_id
                    {
                        cable.end_b.clone()
                    } else {
                        cable.end_a.clone()
                    };
                    self.packet_tap.record(
                        now_ms,
                        cable_id.clone(),
                        from_ep.node_id.clone(),
                        from_ep.port_id.clone(),
                        to_ep.node_id,
                        to_ep.port_id,
                        &frame,
                    );
                    if let Some((direction, deliver_at_ms)) =
                        cable.transmit(&from_ep.node_id, &from_ep.port_id, frame, now_ms)
                    {
                        self.animation_events.push(EdgeAnimationEvent {
                            cable_id: cable_id.clone(),
                            direction,
                            start_ms: now_ms,
                            deliver_at_ms,
                            protocol,
                            size,
                        });
                    }
                }
            }
        }
    }

    pub fn drain_animation_events(&mut self) -> Vec<EdgeAnimationEvent> {
        std::mem::take(&mut self.animation_events)
    }

    pub fn drain_packets(&mut self) -> Vec<CapturedPacket> {
        self.packet_tap.drain_batch()
    }

    pub fn export_pcap(&self) -> Vec<u8> {
        self.packet_tap.export_pcap()
    }

    pub fn clear_packets(&mut self) {
        self.packet_tap.clear();
    }
}

fn classify_frame_protocol(frame: &[u8]) -> String {
    if frame.len() < 14 {
        return "ETH".to_string();
    }
    let ethertype = u16::from_be_bytes([frame[12], frame[13]]);
    match ethertype {
        0x0806 => "ARP".to_string(),
        0x0800 => {
            if frame.len() >= 24 {
                match frame[23] {
                    1 => "ICMP".to_string(),
                    6 => "TCP".to_string(),
                    17 => "UDP".to_string(),
                    _ => "IPv4".to_string(),
                }
            } else {
                "IPv4".to_string()
            }
        }
        0x86DD => "IPv6".to_string(),
        _ => "ETH".to_string(),
    }
}
