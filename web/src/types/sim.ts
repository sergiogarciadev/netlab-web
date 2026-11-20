export type NodeKind = 'host' | 'switch' | 'router';

export type CableDirection = 'AToB' | 'BToA';

export interface EdgeAnimationEvent {
  cable_id: string;
  direction: CableDirection;
  start_ms: number;
  deliver_at_ms: number;
  protocol: string;
  size: number;
}

export interface PingReply {
  from: string;
  ident: number;
  seq: number;
  received_at_ms: number;
}

export interface HostState {
  id: string;
  name: string;
  mac: string;
  ip: string | null;
  prefix_len: number;
  gateway: string | null;
  ping_replies: PingReply[];
}

export interface SwitchState {
  id: string;
  name: string;
  ports: string[];
  mac_table: Record<string, string>;
}

export interface RouterInterfaceState {
  id: string;
  mac: string;
  ip: string | null;
  prefix_len: number;
}

export interface RouteEntry {
  network: string;
  prefix_len: number;
  next_hop: string | null;
  out_iface: string;
}

export interface RouterState {
  id: string;
  name: string;
  interfaces: RouterInterfaceState[];
  routes: RouteEntry[];
}

export interface CableStats {
  id: string;
  packets_sent: number;
  packets_dropped: number;
  in_flight: number;
}

export interface LinkParams {
  latency_ms: number;
  jitter_ms: number;
  loss_percent: number;
  up: boolean;
}

export interface ProtocolLayer {
  name: string;
  fields: [string, string][];
}

export interface PacketSummary {
  src_mac: string;
  dst_mac: string;
  ethertype: string;
  src_ip: string | null;
  dst_ip: string | null;
  protocol: string;
  src_port: number | null;
  dst_port: number | null;
  info: string;
  layers: ProtocolLayer[];
}

export interface CapturedPacket {
  seq: number;
  timestamp_ms: number;
  cable_id: string;
  from_node: string;
  from_port: string;
  to_node: string;
  to_port: string;
  length: number;
  raw: number[];
  summary: PacketSummary;
}

export interface SimulationBatch {
  now_ms: number;
  animations: EdgeAnimationEvent[];
  packets: CapturedPacket[];
  hosts: Record<string, HostState>;
  switches: Record<string, SwitchState>;
  routers: Record<string, RouterState>;
  cables: Record<string, CableStats>;
}

export type WorkerCommand =
  | { type: 'INIT' }
  | { type: 'SET_PLAYING'; playing: boolean }
  | { type: 'SET_SPEED'; speed: number }
  | { type: 'STEP'; delta_ms: number }
  | { type: 'ADD_NODE'; id: string; kind: NodeKind; name: string }
  | { type: 'REMOVE_NODE'; id: string }
  | {
      type: 'CONNECT_NODES';
      cable_id: string;
      node_a: string;
      port_a: string;
      node_b: string;
      port_b: string;
    }
  | { type: 'DISCONNECT_NODES'; cable_id: string }
  | {
      type: 'UPDATE_LINK_PARAMS';
      cable_id: string;
      latency_ms: number;
      jitter_ms: number;
      loss_percent: number;
      up: boolean;
    }
  | {
      type: 'CONFIGURE_INTERFACE';
      node_id: string;
      port_id: string;
      ip: string | null;
      prefix_len: number;
      gateway: string | null;
    }
  | {
      type: 'ADD_ROUTER_ROUTE';
      router_id: string;
      network: string;
      prefix_len: number;
      next_hop: string | null;
      out_iface: string;
    }
  | {
      type: 'SEND_PING';
      host_id: string;
      target_ip: string;
      ident: number;
      seq: number;
    }
  | { type: 'EXPORT_PCAP' }
  | { type: 'CLEAR_PACKETS' };

export type WorkerResponse =
  | { type: 'READY' }
  | { type: 'BATCH'; batch: SimulationBatch }
  | { type: 'PCAP_DATA'; bytes: Uint8Array }
  | { type: 'ERROR'; error: string };
