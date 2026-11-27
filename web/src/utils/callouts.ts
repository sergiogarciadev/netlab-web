import type { CapturedPacket, FirewallState } from '../types/sim';

export interface NodeCalloutData {
  nodeId: string;
  protocol: string;
  title: string;
  message: string;
  tone: 'info' | 'success' | 'warn' | 'danger';
}

function getDhcpMessageType(raw: number[]): number | null {
  // Ethernet (14) + IPv4 (20) + UDP (8) + BOOTP (236) + Magic Cookie (4) = 282
  if (raw.length < 285) return null;
  if (raw[278] !== 99 || raw[279] !== 130 || raw[280] !== 83 || raw[281] !== 99) {
    return null;
  }
  let i = 282;
  while (i < raw.length) {
    const opt = raw[i];
    if (opt === 255) break;
    if (opt === 0) {
      i += 1;
      continue;
    }
    if (i + 1 >= raw.length) break;
    const len = raw[i + 1];
    if (opt === 53 && len >= 1 && i + 2 < raw.length) {
      return raw[i + 2];
    }
    i += 2 + len;
  }
  return null;
}

export function explainPacketForNode(
  pkt: CapturedPacket,
  nodeKind: 'host' | 'switch' | 'router' | 'firewall',
  multiFromSameNode: boolean
): NodeCalloutData {
  const s = pkt.summary;
  const proto = s.protocol;
  const srcIp = s.src_ip ?? s.src_mac;
  const dstIp = s.dst_ip ?? s.dst_mac;

  if (nodeKind === 'switch') {
    if (multiFromSameNode) {
      return {
        nodeId: pkt.from_node,
        protocol: proto,
        title: `L2 Switch (${proto})`,
        message: `Unknown or broadcast MAC (${s.dst_mac}) — flooding frame to all other ports!`,
        tone: 'warn',
      };
    }
    return {
      nodeId: pkt.from_node,
      protocol: proto,
      title: `L2 Switch (${proto})`,
      message: `MAC ${s.dst_mac} known! Forwarding frame out ${pkt.from_port} → ${pkt.to_node}.`,
      tone: 'info',
    };
  }

  if (nodeKind === 'router') {
    return {
      nodeId: pkt.from_node,
      protocol: proto,
      title: `L3 Router (${proto})`,
      message:
        proto === 'ARP' && s.info.includes('is at')
          ? `I am ${srcIp}! Replying with my MAC.`
          : proto === 'ARP'
            ? `Who is ${dstIp}? Resolving next-hop MAC.`
            : `Routing ${proto} packet toward ${dstIp} via ${pkt.from_port} (TTL - 1).`,
      tone: 'info',
    };
  }

  if (nodeKind === 'firewall') {
    return {
      nodeId: pkt.from_node,
      protocol: proto,
      title: `Firewall (${proto})`,
      message:
        proto === 'ARP'
          ? `ARP: ${s.info}`
          : `Inspected ${proto} (${srcIp} → ${dstIp}) — ALLOWED & forwarded.`,
      tone: 'success',
    };
  }

  // Host educational messages by protocol
  switch (proto) {
    case 'ARP': {
      if (s.info.startsWith('Who has')) {
        return {
          nodeId: pkt.from_node,
          protocol: 'ARP',
          title: 'ARP Request',
          message: `Who is ${dstIp}? Tell ${srcIp}!`,
          tone: 'warn',
        };
      }
      return {
        nodeId: pkt.from_node,
        protocol: 'ARP',
        title: 'ARP Reply',
        message: `I am ${srcIp}! My MAC is ${s.src_mac}.`,
        tone: 'success',
      };
    }

    case 'ICMP': {
      if (s.info.includes('Echo (ping) request')) {
        return {
          nodeId: pkt.from_node,
          protocol: 'ICMP',
          title: 'Ping Request',
          message: `Hey ${dstIp}, are you alive?`,
          tone: 'info',
        };
      }
      if (s.info.includes('Echo (ping) reply')) {
        return {
          nodeId: pkt.from_node,
          protocol: 'ICMP',
          title: 'Ping Reply',
          message: `I am alive! (Reply to ${dstIp})`,
          tone: 'success',
        };
      }
      if (s.info.includes('Time-to-live')) {
        return {
          nodeId: pkt.from_node,
          protocol: 'ICMP',
          title: 'TTL Exceeded',
          message: `TTL hit 0 — packet expired in transit!`,
          tone: 'danger',
        };
      }
      return {
        nodeId: pkt.from_node,
        protocol: 'ICMP',
        title: 'ICMP',
        message: s.info,
        tone: 'info',
      };
    }

    case 'DHCP': {
      const msgType = getDhcpMessageType(pkt.raw);
      if (msgType === 1) {
        return {
          nodeId: pkt.from_node,
          protocol: 'DHCP',
          title: 'DHCP Discover',
          message: `Anyone out there? I need an IP address!`,
          tone: 'warn',
        };
      }
      if (msgType === 2) {
        return {
          nodeId: pkt.from_node,
          protocol: 'DHCP',
          title: 'DHCP Offer',
          message: `Here! You can lease this IP configuration.`,
          tone: 'info',
        };
      }
      if (msgType === 3) {
        return {
          nodeId: pkt.from_node,
          protocol: 'DHCP',
          title: 'DHCP Request',
          message: `Yes please, I accept that IP lease!`,
          tone: 'info',
        };
      }
      if (msgType === 5) {
        return {
          nodeId: pkt.from_node,
          protocol: 'DHCP',
          title: 'DHCP Ack',
          message: `Lease confirmed! IP, gateway & DNS assigned.`,
          tone: 'success',
        };
      }
      return {
        nodeId: pkt.from_node,
        protocol: 'DHCP',
        title: 'DHCP',
        message: `Exchanging IP lease configuration.`,
        tone: 'info',
      };
    }

    case 'DNS': {
      if (s.dst_port === 53) {
        return {
          nodeId: pkt.from_node,
          protocol: 'DNS',
          title: 'DNS Query',
          message: `Hey ${dstIp}, what IP address belongs to this hostname?`,
          tone: 'info',
        };
      }
      return {
        nodeId: pkt.from_node,
        protocol: 'DNS',
        title: 'DNS Response',
        message: `Here is the resolved IP address you asked for!`,
        tone: 'success',
      };
    }

    case 'HTTP': {
      if (s.info.startsWith('GET ')) {
        return {
          nodeId: pkt.from_node,
          protocol: 'HTTP',
          title: 'HTTP GET',
          message: `Hey ${dstIp}, please send me ${s.info.split(' ')[1] ?? '/'}.`,
          tone: 'info',
        };
      }
      if (s.info.startsWith('POST ')) {
        return {
          nodeId: pkt.from_node,
          protocol: 'HTTP',
          title: 'HTTP POST',
          message: `Uploading data to ${dstIp} (${s.info.split(' ')[1] ?? '/'}).`,
          tone: 'info',
        };
      }
      if (s.info.includes('200 OK') || s.info.includes('201')) {
        return {
          nodeId: pkt.from_node,
          protocol: 'HTTP',
          title: 'HTTP Response',
          message: `200 OK — here is your requested content!`,
          tone: 'success',
        };
      }
      return {
        nodeId: pkt.from_node,
        protocol: 'HTTP',
        title: 'HTTP',
        message: s.info,
        tone: 'warn',
      };
    }

    case 'TCP': {
      if (s.info.includes('[SYN, ACK]')) {
        return {
          nodeId: pkt.from_node,
          protocol: 'TCP',
          title: 'TCP Handshake (2/3)',
          message: `Sure ${dstIp}, let's connect! (SYN-ACK)`,
          tone: 'success',
        };
      }
      if (s.info.includes('[SYN]')) {
        return {
          nodeId: pkt.from_node,
          protocol: 'TCP',
          title: 'TCP Handshake (1/3)',
          message: `Knock knock ${dstIp}:${s.dst_port ?? ''}! Can we connect? (SYN)`,
          tone: 'info',
        };
      }
      if (s.info.includes('[FIN')) {
        return {
          nodeId: pkt.from_node,
          protocol: 'TCP',
          title: 'TCP Teardown',
          message: `Goodbye ${dstIp}, closing my connection! (FIN)`,
          tone: 'warn',
        };
      }
      if (s.info.includes('[RST')) {
        return {
          nodeId: pkt.from_node,
          protocol: 'TCP',
          title: 'TCP Reset',
          message: `Connection refused or reset! (RST)`,
          tone: 'danger',
        };
      }
      if (s.info.includes('Len=0')) {
        return {
          nodeId: pkt.from_node,
          protocol: 'TCP',
          title: 'TCP ACK',
          message: `Got it, ${dstIp}! Segment acknowledged.`,
          tone: 'info',
        };
      }
      return {
        nodeId: pkt.from_node,
        protocol: 'TCP',
        title: 'TCP Data',
        message: `Sending application data to ${dstIp}:${s.dst_port ?? ''}.`,
        tone: 'info',
      };
    }

    case 'UDP': {
      return {
        nodeId: pkt.from_node,
        protocol: 'UDP',
        title: 'UDP Datagram',
        message: `Sending connectionless datagram to ${dstIp}:${s.dst_port ?? ''}.`,
        tone: 'info',
      };
    }

    default:
      return {
        nodeId: pkt.from_node,
        protocol: proto,
        title: proto,
        message: s.info,
        tone: 'info',
      };
  }
}

export function buildFirewallDropCallout(fw: FirewallState): NodeCalloutData {
  const latest = fw.logs[fw.logs.length - 1] ?? 'Packet dropped by firewall policy.';
  return {
    nodeId: fw.id,
    protocol: 'FIREWALL',
    title: 'Firewall Blocked',
    message: latest.replace(/^\[\d+ms\]\s*/, ''),
    tone: 'danger',
  };
}
