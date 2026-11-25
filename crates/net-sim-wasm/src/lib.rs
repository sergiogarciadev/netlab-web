use std::collections::BTreeMap;
use std::net::Ipv4Addr;

use net_sim_core::{
    CableConfig, CapturedPacket, EdgeAnimationEvent, Endpoint, FirewallAction, FirewallProtocol,
    FirewallRule, NetworkWorld, PingReply, RouteEntry,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    value
        .serialize(&serializer)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPluginDto {
    pub id: String,
    pub name: String,
    pub state: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostStateDto {
    pub id: String,
    pub name: String,
    pub mac: String,
    pub ip: Option<String>,
    pub prefix_len: u8,
    pub gateway: Option<String>,
    pub dns_servers: Vec<String>,
    pub dns_cache: BTreeMap<String, String>,
    pub files: BTreeMap<String, String>,
    pub console_logs: Vec<String>,
    pub plugins: Vec<InstalledPluginDto>,
    pub ping_replies: Vec<PingReply>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchStateDto {
    pub id: String,
    pub name: String,
    pub ports: Vec<String>,
    pub mac_table: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterInterfaceDto {
    pub id: String,
    pub mac: String,
    pub ip: Option<String>,
    pub prefix_len: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterStateDto {
    pub id: String,
    pub name: String,
    pub interfaces: Vec<RouterInterfaceDto>,
    pub routes: Vec<RouteEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallStateDto {
    pub id: String,
    pub name: String,
    pub interfaces: Vec<RouterInterfaceDto>,
    pub routes: Vec<RouteEntry>,
    pub rules: Vec<FirewallRule>,
    pub default_policy: FirewallAction,
    pub packets_allowed: u64,
    pub packets_dropped: u64,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CableStatsDto {
    pub id: String,
    pub packets_sent: u64,
    pub packets_dropped: u64,
    pub in_flight: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationBatchDto {
    pub now_ms: u64,
    pub animations: Vec<EdgeAnimationEvent>,
    pub packets: Vec<CapturedPacket>,
    pub hosts: BTreeMap<String, HostStateDto>,
    pub switches: BTreeMap<String, SwitchStateDto>,
    pub routers: BTreeMap<String, RouterStateDto>,
    pub firewalls: BTreeMap<String, FirewallStateDto>,
    pub cables: BTreeMap<String, CableStatsDto>,
}

fn format_mac(mac: &[u8; 6]) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    )
}

#[wasm_bindgen]
pub struct WasmSimulator {
    world: NetworkWorld,
}

impl Default for WasmSimulator {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmSimulator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            world: NetworkWorld::new(),
        }
    }

    pub fn reset(&mut self) {
        self.world = NetworkWorld::new();
    }

    pub fn add_node(&mut self, id: &str, kind: &str, name: &str) -> Result<(), JsValue> {
        match kind {
            "host" => {
                self.world.add_host(id, name);
                Ok(())
            }
            "switch" => {
                self.world.add_switch(id, name);
                Ok(())
            }
            "router" => {
                self.world.add_router(id, name);
                Ok(())
            }
            "firewall" => {
                self.world.add_firewall(id, name);
                Ok(())
            }
            _ => Err(JsValue::from_str(&format!("Unknown node kind: {kind}"))),
        }
    }

    pub fn remove_node(&mut self, id: &str) {
        self.world.remove_node(id);
    }

    pub fn rename_node(&mut self, id: &str, name: &str) -> Result<(), JsValue> {
        self.world
            .rename_node(id, name)
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn connect_nodes(
        &mut self,
        cable_id: &str,
        node_a: &str,
        port_a: &str,
        node_b: &str,
        port_b: &str,
    ) -> Result<(), JsValue> {
        self.world
            .connect(
                cable_id,
                Endpoint::new(node_a, port_a),
                Endpoint::new(node_b, port_b),
            )
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn disconnect_nodes(&mut self, cable_id: &str) {
        self.world.disconnect(cable_id);
    }

    pub fn update_link_params(
        &mut self,
        cable_id: &str,
        latency_ms: u32,
        jitter_ms: u32,
        loss_percent: f64,
        up: bool,
    ) -> Result<(), JsValue> {
        self.world
            .update_cable_config(
                cable_id,
                CableConfig {
                    latency_ms: latency_ms as u64,
                    jitter_ms: jitter_ms as u64,
                    loss_percent,
                    up,
                },
            )
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn configure_interface(
        &mut self,
        node_id: &str,
        port_id: &str,
        ip: Option<String>,
        prefix_len: u8,
        gateway: Option<String>,
    ) -> Result<(), JsValue> {
        let parsed_ip = match ip {
            Some(s) if !s.trim().is_empty() => Some(
                s.trim()
                    .parse::<Ipv4Addr>()
                    .map_err(|e| JsValue::from_str(&format!("Invalid IP: {e}")))?,
            ),
            _ => None,
        };
        let parsed_gw = match gateway {
            Some(s) if !s.trim().is_empty() => Some(
                s.trim()
                    .parse::<Ipv4Addr>()
                    .map_err(|e| JsValue::from_str(&format!("Invalid gateway: {e}")))?,
            ),
            _ => None,
        };

        if self.world.hosts.contains_key(node_id) {
            self.world
                .configure_host_ip(node_id, parsed_ip, prefix_len, parsed_gw)
                .map_err(|e| JsValue::from_str(&e))
        } else if self.world.routers.contains_key(node_id) {
            self.world
                .configure_router_interface(node_id, port_id, parsed_ip, prefix_len)
                .map_err(|e| JsValue::from_str(&e))
        } else if self.world.firewalls.contains_key(node_id) {
            self.world
                .configure_firewall_interface(node_id, port_id, parsed_ip, prefix_len)
                .map_err(|e| JsValue::from_str(&e))
        } else {
            Err(JsValue::from_str(&format!("Node {node_id} not found or has no L3 interfaces")))
        }
    }

    pub fn add_router_route(
        &mut self,
        router_id: &str,
        network: &str,
        prefix_len: u8,
        next_hop: Option<String>,
        out_iface: &str,
    ) -> Result<(), JsValue> {
        let net: Ipv4Addr = network
            .parse()
            .map_err(|e| JsValue::from_str(&format!("Invalid network IP: {e}")))?;
        let nh = match next_hop {
            Some(s) if !s.trim().is_empty() => Some(
                s.trim()
                    .parse::<Ipv4Addr>()
                    .map_err(|e| JsValue::from_str(&format!("Invalid next_hop: {e}")))?,
            ),
            _ => None,
        };
        let entry = RouteEntry {
            network: net,
            prefix_len,
            next_hop: nh,
            out_iface: out_iface.to_string(),
        };
        if self.world.firewalls.contains_key(router_id) {
            self.world
                .add_firewall_route(router_id, entry)
                .map_err(|e| JsValue::from_str(&e))
        } else {
            self.world
                .add_router_route(router_id, entry)
                .map_err(|e| JsValue::from_str(&e))
        }
    }

    pub fn remove_router_route(&mut self, router_id: &str, index: usize) -> Result<(), JsValue> {
        if self.world.firewalls.contains_key(router_id) {
            self.world
                .remove_firewall_route(router_id, index)
                .map_err(|e| JsValue::from_str(&e))
        } else {
            self.world
                .remove_router_route(router_id, index)
                .map_err(|e| JsValue::from_str(&e))
        }
    }

    pub fn add_firewall_rule(
        &mut self,
        fw_id: &str,
        action: &str,
        protocol: &str,
        src_cidr: Option<String>,
        dst_cidr: Option<String>,
        dst_port: Option<u16>,
    ) -> Result<u64, JsValue> {
        let act = match action.to_lowercase().as_str() {
            "allow" => FirewallAction::Allow,
            "deny" => FirewallAction::Deny,
            _ => return Err(JsValue::from_str(&format!("Invalid action: {action}"))),
        };
        let proto = match protocol.to_uppercase().as_str() {
            "ANY" => FirewallProtocol::Any,
            "ICMP" => FirewallProtocol::ICMP,
            "TCP" => FirewallProtocol::TCP,
            "UDP" => FirewallProtocol::UDP,
            _ => return Err(JsValue::from_str(&format!("Invalid protocol: {protocol}"))),
        };
        self.world
            .add_firewall_rule(fw_id, act, proto, src_cidr, dst_cidr, dst_port)
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn remove_firewall_rule(&mut self, fw_id: &str, rule_id: u64) -> Result<(), JsValue> {
        self.world
            .remove_firewall_rule(fw_id, rule_id)
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn set_firewall_default_policy(&mut self, fw_id: &str, policy: &str) -> Result<(), JsValue> {
        let act = match policy.to_lowercase().as_str() {
            "allow" => FirewallAction::Allow,
            "deny" => FirewallAction::Deny,
            _ => return Err(JsValue::from_str(&format!("Invalid policy: {policy}"))),
        };
        self.world
            .set_firewall_default_policy(fw_id, act)
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn send_ping(
        &mut self,
        host_id: &str,
        target_ip: &str,
        ident: u16,
        seq: u16,
    ) -> Result<(), JsValue> {
        let target: Ipv4Addr = target_ip
            .trim()
            .parse()
            .map_err(|e| JsValue::from_str(&format!("Invalid target IP: {e}")))?;
        self.world
            .send_ping(host_id, target, ident, seq)
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn tick(&mut self, delta_ms: u32) {
        self.world.step(delta_ms as u64);
    }

    pub fn export_pcap(&self) -> Vec<u8> {
        self.world.export_pcap()
    }

    pub fn clear_packets(&mut self) {
        self.world.clear_packets();
    }

    pub fn list_plugins(&self) -> Result<JsValue, JsValue> {
        let list = self.world.plugin_registry.list_metadata();
        to_js(&list)
    }

    pub fn attach_plugin(&mut self, host_id: &str, plugin_id: &str) -> Result<(), JsValue> {
        self.world
            .attach_plugin(host_id, plugin_id)
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn detach_plugin(&mut self, host_id: &str, plugin_id: &str) -> Result<(), JsValue> {
        self.world
            .detach_plugin(host_id, plugin_id)
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn run_plugin_command(
        &mut self,
        host_id: &str,
        plugin_id: &str,
        cmd_json: &str,
    ) -> Result<JsValue, JsValue> {
        let cmd: serde_json::Value = serde_json::from_str(cmd_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid command JSON: {e}")))?;
        let res = self
            .world
            .run_plugin_command(host_id, plugin_id, cmd)
            .map_err(|e| JsValue::from_str(&e))?;
        to_js(&res)
    }

    pub fn set_host_dns(&mut self, host_id: &str, dns_ip: Option<String>) -> Result<(), JsValue> {
        let parsed = match dns_ip {
            Some(s) if !s.trim().is_empty() => Some(
                s.trim()
                    .parse::<Ipv4Addr>()
                    .map_err(|e| JsValue::from_str(&format!("Invalid DNS IP: {e}")))?,
            ),
            _ => None,
        };
        self.world
            .set_host_dns(host_id, parsed)
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn exec_console_command(&mut self, host_id: &str, line: &str) -> Result<String, JsValue> {
        self.world
            .exec_console_command(host_id, line)
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn drain_events(&mut self) -> Result<JsValue, JsValue> {
        let animations = self.world.drain_animation_events();
        let packets = self.world.drain_packets();

        let hosts = self
            .world
            .hosts
            .iter()
            .map(|(id, h)| {
                let dto = HostStateDto {
                    id: id.clone(),
                    name: h.name.clone(),
                    mac: format_mac(&h.mac),
                    ip: h.ip_cidr.map(|(ip, _)| ip.to_string()),
                    prefix_len: h.ip_cidr.map(|(_, p)| p).unwrap_or(24),
                    gateway: h.gateway.map(|g| g.to_string()),
                    dns_servers: h.dns_servers.iter().map(|d| d.to_string()).collect(),
                    dns_cache: h
                        .dns_cache
                        .iter()
                        .map(|(k, v)| (k.clone(), v.to_string()))
                        .collect(),
                    files: h.files.clone(),
                    console_logs: h.console_logs.iter().rev().take(40).rev().cloned().collect(),
                    plugins: h
                        .plugins
                        .iter()
                        .map(|p| InstalledPluginDto {
                            id: p.id().to_string(),
                            name: p.name().to_string(),
                            state: p.get_state(),
                        })
                        .collect(),
                    ping_replies: h.ping_replies.clone(),
                };
                (id.clone(), dto)
            })
            .collect();

        let switches = self
            .world
            .switches
            .iter()
            .map(|(id, sw)| {
                let mac_table = sw
                    .mac_table
                    .iter()
                    .map(|(mac, entry)| (format_mac(mac), entry.port_id.clone()))
                    .collect();
                let dto = SwitchStateDto {
                    id: id.clone(),
                    name: sw.name.clone(),
                    ports: sw.ports.iter().cloned().collect(),
                    mac_table,
                };
                (id.clone(), dto)
            })
            .collect();

        let routers = self
            .world
            .routers
            .iter()
            .map(|(id, r)| {
                let interfaces = r
                    .interfaces
                    .values()
                    .map(|iface| RouterInterfaceDto {
                        id: iface.id.clone(),
                        mac: format_mac(&iface.mac),
                        ip: iface.ip.map(|ip| ip.to_string()),
                        prefix_len: iface.prefix_len,
                    })
                    .collect();
                let dto = RouterStateDto {
                    id: id.clone(),
                    name: r.name.clone(),
                    interfaces,
                    routes: r.routes.clone(),
                };
                (id.clone(), dto)
            })
            .collect();

        let firewalls = self
            .world
            .firewalls
            .iter()
            .map(|(id, fw)| {
                let interfaces = fw
                    .router
                    .interfaces
                    .values()
                    .map(|iface| RouterInterfaceDto {
                        id: iface.id.clone(),
                        mac: format_mac(&iface.mac),
                        ip: iface.ip.map(|ip| ip.to_string()),
                        prefix_len: iface.prefix_len,
                    })
                    .collect();
                let dto = FirewallStateDto {
                    id: id.clone(),
                    name: fw.name.clone(),
                    interfaces,
                    routes: fw.router.routes.clone(),
                    rules: fw.rules.clone(),
                    default_policy: fw.default_policy,
                    packets_allowed: fw.packets_allowed,
                    packets_dropped: fw.packets_dropped,
                    logs: fw.logs.iter().rev().take(30).rev().cloned().collect(),
                };
                (id.clone(), dto)
            })
            .collect();

        let cables = self
            .world
            .cables
            .iter()
            .map(|(id, c)| {
                let dto = CableStatsDto {
                    id: id.clone(),
                    packets_sent: c.packets_sent,
                    packets_dropped: c.packets_dropped,
                    in_flight: c.in_flight_count(),
                };
                (id.clone(), dto)
            })
            .collect();

        let batch = SimulationBatchDto {
            now_ms: self.world.now_ms,
            animations,
            packets,
            hosts,
            switches,
            routers,
            firewalls,
            cables,
        };

        to_js(&batch)
    }
}
