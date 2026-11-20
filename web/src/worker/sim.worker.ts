import init, { WasmSimulator } from '../wasm/net_sim_wasm';
import type { SimulationBatch, WorkerCommand, WorkerResponse } from '../types/sim';

let sim: WasmSimulator | null = null;
let playing = false;
let speed = 1.0;
let msAccumulator = 0;
let loopTimer: ReturnType<typeof setInterval> | null = null;

function post(msg: WorkerResponse) {
  self.postMessage(msg);
}

function flushBatch() {
  if (!sim) return;
  try {
    const batch = sim.drain_events() as SimulationBatch;
    post({ type: 'BATCH', batch });
  } catch (err) {
    post({ type: 'ERROR', error: String(err) });
  }
}

function startLoop() {
  if (loopTimer !== null) return;
  loopTimer = setInterval(() => {
    if (!playing || !sim) return;
    msAccumulator += 33 * speed;
    const wholeMs = Math.floor(msAccumulator);
    if (wholeMs >= 1) {
      msAccumulator -= wholeMs;
      sim.tick(wholeMs);
      flushBatch();
    }
  }, 33);
}

self.onmessage = async (event: MessageEvent<WorkerCommand>) => {
  const cmd = event.data;
  try {
    if (cmd.type === 'INIT') {
      await init();
      sim = new WasmSimulator();
      startLoop();
      post({ type: 'READY' });
      flushBatch();
      return;
    }

    if (!sim) {
      post({ type: 'ERROR', error: 'Simulator not initialized' });
      return;
    }

    switch (cmd.type) {
      case 'SET_PLAYING':
        playing = cmd.playing;
        break;
      case 'SET_SPEED':
        speed = Math.max(0.1, Math.min(10, cmd.speed));
        break;
      case 'STEP':
        sim.tick(cmd.delta_ms);
        flushBatch();
        break;
      case 'ADD_NODE':
        sim.add_node(cmd.id, cmd.kind, cmd.name);
        flushBatch();
        break;
      case 'REMOVE_NODE':
        sim.remove_node(cmd.id);
        flushBatch();
        break;
      case 'CONNECT_NODES':
        sim.connect_nodes(cmd.cable_id, cmd.node_a, cmd.port_a, cmd.node_b, cmd.port_b);
        flushBatch();
        break;
      case 'DISCONNECT_NODES':
        sim.disconnect_nodes(cmd.cable_id);
        flushBatch();
        break;
      case 'UPDATE_LINK_PARAMS':
        sim.update_link_params(
          cmd.cable_id,
          cmd.latency_ms,
          cmd.jitter_ms,
          cmd.loss_percent,
          cmd.up
        );
        flushBatch();
        break;
      case 'CONFIGURE_INTERFACE':
        sim.configure_interface(cmd.node_id, cmd.port_id, cmd.ip, cmd.prefix_len, cmd.gateway);
        flushBatch();
        break;
      case 'ADD_ROUTER_ROUTE':
        sim.add_router_route(
          cmd.router_id,
          cmd.network,
          cmd.prefix_len,
          cmd.next_hop,
          cmd.out_iface
        );
        flushBatch();
        break;
      case 'SEND_PING':
        sim.send_ping(cmd.host_id, cmd.target_ip, cmd.ident, cmd.seq);
        flushBatch();
        break;
      case 'EXPORT_PCAP': {
        const bytes = sim.export_pcap();
        post({ type: 'PCAP_DATA', bytes });
        break;
      }
      case 'CLEAR_PACKETS':
        sim.clear_packets();
        break;
    }
  } catch (err) {
    post({ type: 'ERROR', error: String(err) });
  }
};
