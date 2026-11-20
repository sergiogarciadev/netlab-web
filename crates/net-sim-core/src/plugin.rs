use std::collections::BTreeMap;
use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use smoltcp::iface::{Interface, SocketSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFieldSchema {
    pub key: String,
    pub label: String,
    pub field_type: String, // "text" | "number" | "textarea" | "boolean"
    pub default_value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginActionSchema {
    pub command: String,
    pub label: String,
    pub fields: Vec<ConfigFieldSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub config_schema: Vec<ConfigFieldSchema>,
    pub actions: Vec<PluginActionSchema>,
}

pub struct PluginContext<'a> {
    pub host_id: &'a str,
    pub now_ms: u64,
    pub iface: &'a mut Interface,
    pub sockets: &'a mut SocketSet<'static>,
    pub ip_cidr: &'a mut Option<(Ipv4Addr, u8)>,
    pub gateway: &'a mut Option<Ipv4Addr>,
    pub dns_servers: &'a mut Vec<Ipv4Addr>,
    pub dns_cache: &'a mut BTreeMap<String, Ipv4Addr>,
    pub files: &'a mut BTreeMap<String, String>,
    pub console_logs: &'a mut Vec<String>,
}

pub trait AppPlugin: Send {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn on_mount(&mut self, ctx: &mut PluginContext);
    fn on_unmount(&mut self, _ctx: &mut PluginContext) {}
    fn poll(&mut self, ctx: &mut PluginContext);
    fn handle_command(&mut self, cmd: Value, ctx: &mut PluginContext) -> Result<Value, String>;
    fn get_state(&self) -> Value;
}

pub type PluginFactory = fn() -> Box<dyn AppPlugin>;

pub struct PluginEntry {
    pub metadata: PluginMetadata,
    pub factory: PluginFactory,
}

pub struct PluginRegistry {
    entries: BTreeMap<String, PluginEntry>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, metadata: PluginMetadata, factory: PluginFactory) {
        self.entries.insert(
            metadata.id.clone(),
            PluginEntry { metadata, factory },
        );
    }

    pub fn list_metadata(&self) -> Vec<PluginMetadata> {
        self.entries.values().map(|e| e.metadata.clone()).collect()
    }

    pub fn create(&self, id: &str) -> Result<Box<dyn AppPlugin>, String> {
        self.entries
            .get(id)
            .map(|e| (e.factory)())
            .ok_or_else(|| format!("Unknown plugin: {id}"))
    }
}
