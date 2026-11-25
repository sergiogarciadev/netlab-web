import type { FirewallAction, FirewallProtocol, LinkParams, NodeKind } from '../types/sim';

export interface SnapshotPlugin {
  id: string;
  config?: Record<string, unknown>;
}

export interface SnapshotNode {
  id: string;
  kind: NodeKind;
  name: string;
  position: { x: number; y: number };
  ip?: string | null;
  prefix_len?: number;
  gateway?: string | null;
  dns?: string | null;
  router_interfaces?: Array<{
    id: string;
    ip: string | null;
    prefix_len: number;
  }>;
  firewall_policy?: FirewallAction;
  firewall_rules?: Array<{
    action: FirewallAction;
    protocol: FirewallProtocol;
    src_cidr: string | null;
    dst_cidr: string | null;
    dst_port: number | null;
  }>;
  plugins?: SnapshotPlugin[];
}

export interface SnapshotEdge {
  id: string;
  source: string;
  portA: string;
  target: string;
  portB: string;
  params: LinkParams;
}

export interface TopologySnapshot {
  version: 1;
  name: string;
  description?: string;
  nodes: SnapshotNode[];
  edges: SnapshotEdge[];
  initialPings?: Array<{ host_id: string; target_ip: string }>;
}

export const STARTER_LABS: TopologySnapshot[] = [
  {
    version: 1,
    name: 'Lab 1: ARP, Ping & Switch MAC Learning',
    description:
      'Three hosts connected to an L2 VirtualSwitch. Watch ARP broadcast flooding and MAC table learning.',
    nodes: [
      {
        id: 'h1',
        kind: 'host',
        name: 'Host A',
        position: { x: 80, y: 80 },
        ip: '192.168.1.10',
        prefix_len: 24,
      },
      {
        id: 'h2',
        kind: 'host',
        name: 'Host B',
        position: { x: 80, y: 260 },
        ip: '192.168.1.20',
        prefix_len: 24,
      },
      {
        id: 'sw1',
        kind: 'switch',
        name: 'Core Switch',
        position: { x: 350, y: 170 },
      },
      {
        id: 'h3',
        kind: 'host',
        name: 'Host C',
        position: { x: 620, y: 170 },
        ip: '192.168.1.30',
        prefix_len: 24,
      },
    ],
    edges: [
      {
        id: 'c1',
        source: 'h1',
        portA: 'eth0',
        target: 'sw1',
        portB: 'p1',
        params: { latency_ms: 8, jitter_ms: 0, loss_percent: 0, up: true },
      },
      {
        id: 'c2',
        source: 'h2',
        portA: 'eth0',
        target: 'sw1',
        portB: 'p2',
        params: { latency_ms: 8, jitter_ms: 0, loss_percent: 0, up: true },
      },
      {
        id: 'c3',
        source: 'sw1',
        portA: 'p3',
        target: 'h3',
        portB: 'eth0',
        params: { latency_ms: 8, jitter_ms: 0, loss_percent: 0, up: true },
      },
    ],
    initialPings: [{ host_id: 'h1', target_ip: '192.168.1.30' }],
  },
  {
    version: 1,
    name: 'Lab 2: Multi-Subnet Routing with DHCP & DNS',
    description:
      'Client boots via DHCP on 10.0.1.0/24, crosses VirtualRouter r1 to reach DNS & Web server on 10.0.2.0/24.',
    nodes: [
      {
        id: 'h1',
        kind: 'host',
        name: 'Client PC',
        position: { x: 60, y: 80 },
        plugins: [{ id: 'dhcp-client' }, { id: 'dns-resolver' }, { id: 'http-client' }],
      },
      {
        id: 'h2',
        kind: 'host',
        name: 'DHCP Server',
        position: { x: 60, y: 270 },
        ip: '10.0.1.2',
        prefix_len: 24,
        gateway: '10.0.1.1',
        plugins: [
          {
            id: 'dhcp-server',
            config: {
              pool_start: '10.0.1.100',
              pool_end: '10.0.1.150',
              prefix_len: 24,
              gateway: '10.0.1.1',
              dns_server: '10.0.2.10',
            },
          },
        ],
      },
      {
        id: 'sw1',
        kind: 'switch',
        name: 'LAN Switch',
        position: { x: 290, y: 170 },
      },
      {
        id: 'r1',
        kind: 'router',
        name: 'Gateway Router',
        position: { x: 510, y: 170 },
        router_interfaces: [
          { id: 'eth0', ip: '10.0.1.1', prefix_len: 24 },
          { id: 'eth1', ip: '10.0.2.1', prefix_len: 24 },
        ],
      },
      {
        id: 'h3',
        kind: 'host',
        name: 'DNS + Web Server',
        position: { x: 740, y: 170 },
        ip: '10.0.2.10',
        prefix_len: 24,
        gateway: '10.0.2.1',
        plugins: [
          {
            id: 'dns-server',
            config: {
              records_text: 'web.local A 10.0.2.10\nwww.web.local CNAME web.local',
            },
          },
          { id: 'http-server' },
        ],
      },
    ],
    edges: [
      {
        id: 'c1',
        source: 'h1',
        portA: 'eth0',
        target: 'sw1',
        portB: 'p1',
        params: { latency_ms: 5, jitter_ms: 0, loss_percent: 0, up: true },
      },
      {
        id: 'c2',
        source: 'h2',
        portA: 'eth0',
        target: 'sw1',
        portB: 'p2',
        params: { latency_ms: 5, jitter_ms: 0, loss_percent: 0, up: true },
      },
      {
        id: 'c3',
        source: 'sw1',
        portA: 'p3',
        target: 'r1',
        portB: 'eth0',
        params: { latency_ms: 5, jitter_ms: 0, loss_percent: 0, up: true },
      },
      {
        id: 'c4',
        source: 'r1',
        portA: 'eth1',
        target: 'h3',
        portB: 'eth0',
        params: { latency_ms: 10, jitter_ms: 0, loss_percent: 0, up: true },
      },
    ],
  },
  {
    version: 1,
    name: 'Lab 3: Enterprise Stack (Web + File + Mail + Packet Loss)',
    description:
      'Full application stack with a degraded WAN link (25% packet loss, 60ms latency) to inspect TCP retransmissions.',
    nodes: [
      {
        id: 'h1',
        kind: 'host',
        name: 'Workstation',
        position: { x: 70, y: 160 },
        ip: '10.10.1.10',
        prefix_len: 24,
        gateway: '10.10.1.1',
        dns: '10.10.2.10',
        plugins: [
          { id: 'dns-resolver' },
          { id: 'http-client' },
          { id: 'file-client' },
          { id: 'mail-client' },
        ],
      },
      {
        id: 'r1',
        kind: 'router',
        name: 'WAN Router',
        position: { x: 310, y: 160 },
        router_interfaces: [
          { id: 'eth0', ip: '10.10.1.1', prefix_len: 24 },
          { id: 'eth1', ip: '10.10.2.1', prefix_len: 24 },
        ],
      },
      {
        id: 'sw1',
        kind: 'switch',
        name: 'DC Switch',
        position: { x: 540, y: 160 },
      },
      {
        id: 'h2',
        kind: 'host',
        name: 'Web & File Server',
        position: { x: 770, y: 70 },
        ip: '10.10.2.10',
        prefix_len: 24,
        gateway: '10.10.2.1',
        plugins: [
          {
            id: 'dns-server',
            config: {
              records_text: 'web.corp A 10.10.2.10\nfiles.corp A 10.10.2.10\nmail.corp A 10.10.2.25',
            },
          },
          { id: 'http-server' },
          { id: 'file-server' },
        ],
      },
      {
        id: 'h3',
        kind: 'host',
        name: 'Mail Server',
        position: { x: 770, y: 260 },
        ip: '10.10.2.25',
        prefix_len: 24,
        gateway: '10.10.2.1',
        plugins: [{ id: 'mail-server' }],
      },
    ],
    edges: [
      {
        id: 'c1',
        source: 'h1',
        portA: 'eth0',
        target: 'r1',
        portB: 'eth0',
        params: { latency_ms: 5, jitter_ms: 0, loss_percent: 0, up: true },
      },
      {
        id: 'c2',
        source: 'r1',
        portA: 'eth1',
        target: 'sw1',
        portB: 'p1',
        params: { latency_ms: 60, jitter_ms: 15, loss_percent: 25, up: true },
      },
      {
        id: 'c3',
        source: 'sw1',
        portA: 'p2',
        target: 'h2',
        portB: 'eth0',
        params: { latency_ms: 5, jitter_ms: 0, loss_percent: 0, up: true },
      },
      {
        id: 'c4',
        source: 'sw1',
        portA: 'p3',
        target: 'h3',
        portB: 'eth0',
        params: { latency_ms: 5, jitter_ms: 0, loss_percent: 0, up: true },
      },
    ],
    initialPings: [{ host_id: 'h1', target_ip: '10.10.2.10' }],
  },
];
