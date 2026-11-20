# NetLab — Browser-Based Network Simulator

A modular, browser-based network topology and packet-level simulator powered by **React 19 + TypeScript + React Flow + Rust/WASM (`smoltcp` + `etherparse`)** running inside a dedicated Web Worker.

## Architecture

- **`crates/net-sim-core`**: Pure Rust simulation engine.
  - `VirtualDevice`: `smoltcp::phy::Device` backed by `VecDeque<Vec<u8>>` RX/TX queues.
  - `VirtualCable`: Point-to-point bidirectional discrete-event link with latency, jitter, packet loss (%), and link up/down state.
  - `VirtualSwitch`: Layer 2 MAC-learning bridge with aging TTL and unicast/flood forwarding.
  - `VirtualRouter`: Multi-interface Layer 3 IPv4 router with longest-prefix match routing table, TTL decrement, ICMP error/echo handling, and per-interface ARP resolution.
  - `PacketTap`: Records raw Ethernet frames on every link, dissects headers via `etherparse`, and exports standard Libpcap (`.pcap`) files for Wireshark.
  - `AppPlugin` & `PluginRegistry`: Modular application plugin architecture (`dhcp-server`, `dhcp-client`, `dns-server`, `dns-resolver`, `http-server`, `http-client`, `file-server`, `file-client`, `mail-server`, `mail-client`).
- **`crates/net-sim-wasm`**: `wasm-bindgen` wrapper exposing `WasmSimulator` with batched ~30–60 FPS telemetry (`drain_events`) so the JS↔WASM boundary is never crossed per packet.
- **`web/`**: Vite + React 19 + Tailwind CSS + `@xyflow/react` + `zustand` frontend with `sim.worker.ts` running the deterministic virtual clock (`0.1x`–`10x`, Pause, Single-Step).

## Running Locally

```bash
# Run all Rust core & plugin integration tests
cargo test

# Build the WASM package
wasm-pack build crates/net-sim-wasm --target web --out-dir ../../web/src/wasm

# Start the frontend dev server
cd web
npm install
npm run dev
```

## Creating & Registering a New `AppPlugin` in Rust

1. Implement the `AppPlugin` trait in `crates/net-sim-core/src/plugins/my_plugin.rs`:

```rust
use serde_json::{json, Value};
use crate::plugin::{AppPlugin, PluginContext, PluginMetadata};

pub struct MyPlugin;

impl MyPlugin {
    pub fn metadata() -> PluginMetadata {
        PluginMetadata {
            id: "my-plugin".into(),
            name: "My Custom Daemon".into(),
            description: "Describes what the plugin does.".into(),
            config_schema: vec![],
            actions: vec![],
        }
    }
}

impl AppPlugin for MyPlugin {
    fn id(&self) -> &'static str { "my-plugin" }
    fn name(&self) -> &'static str { "My Custom Daemon" }

    fn on_mount(&mut self, ctx: &mut PluginContext) {
        // Bind sockets on ctx.sockets here
    }

    fn poll(&mut self, ctx: &mut PluginContext) {
        // Process socket RX/TX on every simulation tick
    }

    fn handle_command(&mut self, cmd: Value, ctx: &mut PluginContext) -> Result<Value, String> {
        Ok(json!({ "ok": true }))
    }

    fn get_state(&self) -> Value {
        json!({})
    }
}
```

2. Register it in `crates/net-sim-core/src/plugins/mod.rs`:

```rust
registry.register(my_plugin::MyPlugin::metadata(), || Box::new(my_plugin::MyPlugin));
```

3. Rebuild the WASM package (`wasm-pack build ...`). The new plugin's configuration and action forms will automatically appear in the **Plugins** tab of the Node Inspector.

## Keyboard Shortcuts

- `Space`: Play / Pause simulation
- `S`: Single-step `+10ms` (when paused)
- `Delete` / `Backspace`: Remove selected node or link
