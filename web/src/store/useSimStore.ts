import { create } from 'zustand';
import {
  addEdge,
  applyEdgeChanges,
  applyNodeChanges,
  type Connection,
  type Edge,
  type EdgeChange,
  type Node,
  type NodeChange,
} from '@xyflow/react';
import type {
  CableStats,
  CapturedPacket,
  EdgeAnimationEvent,
  HostState,
  LinkParams,
  NodeKind,
  PluginMetadata,
  RouterState,
  SimulationBatch,
  SwitchState,
  WorkerCommand,
  WorkerResponse,
} from '../types/sim';

export interface ActivePacketDot {
  id: string;
  cable_id: string;
  direction: 'AToB' | 'BToA';
  protocol: string;
  createdAtPerfMs: number;
  durationMs: number;
}

export interface NetNodeData extends Record<string, unknown> {
  kind: NodeKind;
  label: string;
}

export interface NetEdgeData extends Record<string, unknown> {
  portA: string;
  portB: string;
  params: LinkParams;
}

interface SimStore {
  worker: Worker | null;
  ready: boolean;
  playing: boolean;
  speed: number;
  nowMs: number;
  lastError: string | null;

  nodes: Node<NetNodeData>[];
  edges: Edge<NetEdgeData>[];
  selectedNodeId: string | null;
  selectedEdgeId: string | null;

  hosts: Record<string, HostState>;
  switches: Record<string, SwitchState>;
  routers: Record<string, RouterState>;
  cables: Record<string, CableStats>;
  activePackets: ActivePacketDot[];
  packets: CapturedPacket[];
  pluginCatalog: PluginMetadata[];
  consoleHistory: Record<string, string[]>;

  nextNodeSeq: Record<NodeKind, number>;
  nextCableSeq: number;
  nextPingSeq: number;

  initWorker: () => void;
  postCommand: (cmd: WorkerCommand) => void;
  setPlaying: (playing: boolean) => void;
  setSpeed: (speed: number) => void;
  stepOnce: (deltaMs?: number) => void;

  onNodesChange: (changes: NodeChange<Node<NetNodeData>>[]) => void;
  onEdgesChange: (changes: EdgeChange<Edge<NetEdgeData>>[]) => void;
  onConnect: (connection: Connection) => void;

  addNode: (kind: NodeKind, position?: { x: number; y: number }) => string;
  removeNode: (id: string) => void;
  removeEdge: (id: string) => void;
  selectNode: (id: string | null) => void;
  selectEdge: (id: string | null) => void;

  configureInterface: (
    nodeId: string,
    portId: string,
    ip: string | null,
    prefixLen: number,
    gateway: string | null
  ) => void;
  updateLinkParams: (cableId: string, params: LinkParams) => void;
  sendPing: (hostId: string, targetIp: string) => void;
  exportPcap: () => void;
  clearPackets: () => void;
  attachPlugin: (hostId: string, pluginId: string) => void;
  detachPlugin: (hostId: string, pluginId: string) => void;
  runPluginCommand: (hostId: string, pluginId: string, cmd: Record<string, unknown>) => void;
  setHostDns: (hostId: string, dnsIp: string | null) => void;
  execConsole: (hostId: string, line: string) => void;
  clearError: () => void;
}

function allocatePortForNode(
  nodeId: string,
  kind: NodeKind,
  edges: Edge<NetEdgeData>[]
): string {
  if (kind === 'host') {
    return 'eth0';
  }
  const usedPorts = new Set<string>();
  for (const e of edges) {
    if (e.source === nodeId && e.data?.portA) usedPorts.add(e.data.portA);
    if (e.target === nodeId && e.data?.portB) usedPorts.add(e.data.portB);
  }
  const prefix = kind === 'switch' ? 'p' : 'eth';
  let idx = kind === 'switch' ? 1 : 0;
  while (usedPorts.has(`${prefix}${idx}`)) {
    idx++;
  }
  return `${prefix}${idx}`;
}

export const useSimStore = create<SimStore>((set, get) => ({
  worker: null,
  ready: false,
  playing: false,
  speed: 1,
  nowMs: 0,
  lastError: null,

  nodes: [],
  edges: [],
  selectedNodeId: null,
  selectedEdgeId: null,

  hosts: {},
  switches: {},
  routers: {},
  cables: {},
  activePackets: [],
  packets: [],
  pluginCatalog: [],
  consoleHistory: {},

  nextNodeSeq: { host: 1, switch: 1, router: 1 },
  nextCableSeq: 1,
  nextPingSeq: 1,

  initWorker: () => {
    if (get().worker) return;
    const worker = new Worker(new URL('../worker/sim.worker.ts', import.meta.url), {
      type: 'module',
    });

    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      const msg = event.data;
      if (msg.type === 'READY') {
        set({ ready: true });
      } else if (msg.type === 'PLUGIN_CATALOG') {
        set({ pluginCatalog: msg.plugins });
      } else if (msg.type === 'CONSOLE_OUTPUT') {
        set((state) => {
          const prev = state.consoleHistory[msg.host_id] ?? [];
          const next = [...prev, `$ ${msg.line}`, ...(msg.output ? [msg.output] : [])].slice(-80);
          return {
            consoleHistory: { ...state.consoleHistory, [msg.host_id]: next },
          };
        });
      } else if (msg.type === 'ERROR') {
        set({ lastError: msg.error });
      } else if (msg.type === 'PCAP_DATA') {
        const blob = new Blob([new Uint8Array(msg.bytes)], {
          type: 'application/vnd.tcpdump.pcap',
        });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `netlab-capture-${Date.now()}.pcap`;
        a.click();
        URL.revokeObjectURL(url);
      } else if (msg.type === 'BATCH') {
        const batch: SimulationBatch = msg.batch;
        const nowPerf = performance.now();
        const speed = get().speed;

        const newDots: ActivePacketDot[] = batch.animations.map(
          (anim: EdgeAnimationEvent, i: number) => {
            const simDur = Math.max(1, anim.deliver_at_ms - anim.start_ms);
            // Visual duration scaled so packets are clearly visible (at least 350ms, max 1400ms)
            const visualDur = Math.min(1400, Math.max(350, (simDur * 25) / Math.max(0.25, speed)));
            return {
              id: `${anim.cable_id}-${anim.start_ms}-${i}-${nowPerf}`,
              cable_id: anim.cable_id,
              direction: anim.direction,
              protocol: anim.protocol,
              createdAtPerfMs: nowPerf,
              durationMs: visualDur,
            };
          }
        );

        set((state) => {
          const validExisting = state.activePackets.filter(
            (p) => nowPerf - p.createdAtPerfMs < p.durationMs + 50
          );
          const mergedPackets =
            batch.packets && batch.packets.length > 0
              ? [...state.packets, ...batch.packets].slice(-2000)
              : state.packets;
          return {
            nowMs: batch.now_ms,
            hosts: batch.hosts,
            switches: batch.switches,
            routers: batch.routers,
            cables: batch.cables,
            activePackets: [...validExisting, ...newDots].slice(-120),
            packets: mergedPackets,
          };
        });
      }
    };

    worker.postMessage({ type: 'INIT' } satisfies WorkerCommand);
    set({ worker });
  },

  postCommand: (cmd) => {
    const w = get().worker;
    if (w) w.postMessage(cmd);
  },

  setPlaying: (playing) => {
    set({ playing });
    get().postCommand({ type: 'SET_PLAYING', playing });
  },

  setSpeed: (speed) => {
    set({ speed });
    get().postCommand({ type: 'SET_SPEED', speed });
  },

  stepOnce: (deltaMs = 10) => {
    get().postCommand({ type: 'STEP', delta_ms: deltaMs });
  },

  onNodesChange: (changes) => {
    set((state) => ({
      nodes: applyNodeChanges(changes, state.nodes),
    }));
  },

  onEdgesChange: (changes) => {
    set((state) => ({
      edges: applyEdgeChanges(changes, state.edges),
    }));
  },

  onConnect: (connection) => {
    if (!connection.source || !connection.target || connection.source === connection.target) {
      return;
    }
    const state = get();
    const srcNode = state.nodes.find((n) => n.id === connection.source);
    const dstNode = state.nodes.find((n) => n.id === connection.target);
    if (!srcNode || !dstNode) return;

    const portA = allocatePortForNode(srcNode.id, srcNode.data.kind, state.edges);
    const portB = allocatePortForNode(dstNode.id, dstNode.data.kind, state.edges);
    const cableId = `c${state.nextCableSeq}`;
    const params: LinkParams = { latency_ms: 5, jitter_ms: 0, loss_percent: 0, up: true };

    const newEdge: Edge<NetEdgeData> = {
      id: cableId,
      source: srcNode.id,
      target: dstNode.id,
      type: 'packetEdge',
      data: { portA, portB, params },
    };

    set({
      edges: addEdge(newEdge, state.edges),
      nextCableSeq: state.nextCableSeq + 1,
    });

    state.postCommand({
      type: 'CONNECT_NODES',
      cable_id: cableId,
      node_a: srcNode.id,
      port_a: portA,
      node_b: dstNode.id,
      port_b: portB,
    });
  },

  addNode: (kind, position) => {
    const state = get();
    const seq = state.nextNodeSeq[kind];
    const prefix = kind === 'host' ? 'h' : kind === 'switch' ? 'sw' : 'r';
    const id = `${prefix}${seq}`;
    const labelPrefix = kind === 'host' ? 'Host' : kind === 'switch' ? 'Switch' : 'Router';
    const name = `${labelPrefix} ${seq}`;

    const pos = position ?? {
      x: 180 + ((seq - 1) % 4) * 200,
      y: 120 + Math.floor((seq - 1) / 4) * 150,
    };

    const newNode: Node<NetNodeData> = {
      id,
      type: kind,
      position: pos,
      data: { kind, label: name },
    };

    set({
      nodes: [...state.nodes, newNode],
      nextNodeSeq: { ...state.nextNodeSeq, [kind]: seq + 1 },
      selectedNodeId: id,
      selectedEdgeId: null,
    });

    state.postCommand({ type: 'ADD_NODE', id, kind, name });
    return id;
  },

  removeNode: (id) => {
    const state = get();
    set({
      nodes: state.nodes.filter((n) => n.id !== id),
      edges: state.edges.filter((e) => e.source !== id && e.target !== id),
      selectedNodeId: state.selectedNodeId === id ? null : state.selectedNodeId,
    });
    state.postCommand({ type: 'REMOVE_NODE', id });
  },

  removeEdge: (id) => {
    const state = get();
    set({
      edges: state.edges.filter((e) => e.id !== id),
      selectedEdgeId: state.selectedEdgeId === id ? null : state.selectedEdgeId,
    });
    state.postCommand({ type: 'DISCONNECT_NODES', cable_id: id });
  },

  selectNode: (id) => set({ selectedNodeId: id, selectedEdgeId: id ? null : get().selectedEdgeId }),
  selectEdge: (id) => set({ selectedEdgeId: id, selectedNodeId: id ? null : get().selectedNodeId }),

  configureInterface: (nodeId, portId, ip, prefixLen, gateway) => {
    get().postCommand({
      type: 'CONFIGURE_INTERFACE',
      node_id: nodeId,
      port_id: portId,
      ip,
      prefix_len: prefixLen,
      gateway,
    });
  },

  updateLinkParams: (cableId, params) => {
    set((state) => ({
      edges: state.edges.map((e) =>
        e.id === cableId && e.data ? { ...e, data: { ...e.data, params } } : e
      ),
    }));
    get().postCommand({
      type: 'UPDATE_LINK_PARAMS',
      cable_id: cableId,
      ...params,
    });
  },

  sendPing: (hostId, targetIp) => {
    const seq = get().nextPingSeq;
    set({ nextPingSeq: seq + 1 });
    get().postCommand({
      type: 'SEND_PING',
      host_id: hostId,
      target_ip: targetIp,
      ident: 0x1234,
      seq,
    });
  },

  exportPcap: () => {
    get().postCommand({ type: 'EXPORT_PCAP' });
  },

  clearPackets: () => {
    set({ packets: [] });
    get().postCommand({ type: 'CLEAR_PACKETS' });
  },

  attachPlugin: (hostId, pluginId) => {
    get().postCommand({ type: 'ATTACH_PLUGIN', host_id: hostId, plugin_id: pluginId });
  },

  detachPlugin: (hostId, pluginId) => {
    get().postCommand({ type: 'DETACH_PLUGIN', host_id: hostId, plugin_id: pluginId });
  },

  runPluginCommand: (hostId, pluginId, cmd) => {
    get().postCommand({
      type: 'RUN_PLUGIN_COMMAND',
      host_id: hostId,
      plugin_id: pluginId,
      cmd_json: JSON.stringify(cmd),
    });
  },

  setHostDns: (hostId, dnsIp) => {
    get().postCommand({ type: 'SET_HOST_DNS', host_id: hostId, dns_ip: dnsIp });
  },

  execConsole: (hostId, line) => {
    get().postCommand({ type: 'EXEC_CONSOLE', host_id: hostId, line });
  },

  clearError: () => set({ lastError: null }),
}));
