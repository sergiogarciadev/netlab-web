export type NodeKind = 'host' | 'switch' | 'router' | 'firewall';

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

export interface ConfigFieldSchema {
  key: string;
  label: string;
  field_type: string;
  default_value: unknown;
}

export interface PluginActionSchema {
  command: string;
  label: string;
  fields: ConfigFieldSchema[];
}

export interface PluginMetadata {
  id: string;
  name: string;
  description: string;
  config_schema: ConfigFieldSchema[];
  actions: PluginActionSchema[];
}

export interface InstalledPlugin {
  id: string;
  name: string;
  state: Record<string, unknown>;
}

export interface HostState {
  id: string;
  name: string;
  mac: string;
  ip: string | null;
  prefix_len: number;
  gateway: string | null;
  dns_servers: string[];
  dns_cache: Record<string, string>;
  files: Record<string, string>;
  console_logs: string[];
  plugins: InstalledPlugin[];
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

export type FirewallAction = 'Allow' | 'Deny';
export type FirewallProtocol = 'Any' | 'ICMP' | 'TCP' | 'UDP';

export interface FirewallRule {
  id: number;
  action: FirewallAction;
  protocol: FirewallProtocol;
  src_cidr: string | null;
  dst_cidr: string | null;
  dst_port: number | null;
  hits: number;
}

export interface FirewallState {
  id: string;
  name: string;
  interfaces: RouterInterfaceState[];
  routes: RouteEntry[];
  rules: FirewallRule[];
  default_policy: FirewallAction;
  packets_allowed: number;
  packets_dropped: number;
  logs: string[];
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
  firewalls: Record<string, FirewallState>;
  cables: Record<string, CableStats>;
}

export type WorkerCommand =
  | { type: 'INIT' }
  | { type: 'RESET_WORLD' }
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
      type: 'REMOVE_ROUTER_ROUTE';
      router_id: string;
      index: number;
    }
  | {
      type: 'ADD_FIREWALL_RULE';
      fw_id: string;
      action: string;
      protocol: string;
      src_cidr: string | null;
      dst_cidr: string | null;
      dst_port: number | null;
    }
  | { type: 'REMOVE_FIREWALL_RULE'; fw_id: string; rule_id: number }
  | { type: 'SET_FIREWALL_POLICY'; fw_id: string; policy: string }
  | {
      type: 'SEND_PING';
      host_id: string;
      target_ip: string;
      ident: number;
      seq: number;
    }
  | { type: 'EXPORT_PCAP' }
  | { type: 'CLEAR_PACKETS' }
  | { type: 'ATTACH_PLUGIN'; host_id: string; plugin_id: string }
  | { type: 'DETACH_PLUGIN'; host_id: string; plugin_id: string }
  | {
      type: 'RUN_PLUGIN_COMMAND';
      host_id: string;
      plugin_id: string;
      cmd_json: string;
    }
  | { type: 'SET_HOST_DNS'; host_id: string; dns_ip: string | null }
  | { type: 'EXEC_CONSOLE'; host_id: string; line: string };

export type WorkerResponse =
  | { type: 'READY' }
  | { type: 'PLUGIN_CATALOG'; plugins: PluginMetadata[] }
  | { type: 'BATCH'; batch: SimulationBatch }
  | { type: 'PCAP_DATA'; bytes: Uint8Array }
  | { type: 'CONSOLE_OUTPUT'; host_id: string; line: string; output: string }
  | { type: 'ERROR'; error: string };
