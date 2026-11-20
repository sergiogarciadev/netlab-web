use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};
use smoltcp::iface::{Config, Interface, SocketHandle, SocketSet};
use smoltcp::phy::ChecksumCapabilities;
use smoltcp::socket::icmp;
use smoltcp::time::Instant;
use smoltcp::wire::{
    EthernetAddress, HardwareAddress, Icmpv4Packet, Icmpv4Repr, IpAddress, IpCidr, Ipv4Address,
    Ipv4Cidr,
};

use crate::device::VirtualDevice;
use crate::switch::MacAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingReply {
    pub from: Ipv4Addr,
    pub ident: u16,
    pub seq: u16,
    pub received_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPing {
    pub target: Ipv4Addr,
    pub ident: u16,
    pub seq: u16,
    pub sent_at_ms: u64,
}

/// Simulated network endpoint powered by `smoltcp`.
pub struct VirtualHost {
    pub id: String,
    pub name: String,
    pub mac: MacAddr,
    pub ip_cidr: Option<(Ipv4Addr, u8)>,
    pub gateway: Option<Ipv4Addr>,
    pub device: VirtualDevice,
    pub iface: Interface,
    pub sockets: SocketSet<'static>,
    pub ping_socket: SocketHandle,
    pub pending_pings: Vec<PendingPing>,
    pub ping_replies: Vec<PingReply>,
}

impl VirtualHost {
    pub fn new(id: impl Into<String>, name: impl Into<String>, mac: MacAddr, now_ms: u64) -> Self {
        let mut device = VirtualDevice::default();
        let hw = HardwareAddress::Ethernet(EthernetAddress(mac));
        let mut config = Config::new(hw);
        config.random_seed = u64::from_be_bytes([mac[0], mac[1], mac[2], mac[3], mac[4], mac[5], 0x13, 0x37]);

        let iface = Interface::new(config, &mut device, Instant::from_millis(now_ms as i64));
        let mut sockets = SocketSet::new(vec![]);

        let rx_buf = icmp::PacketBuffer::new(
            vec![icmp::PacketMetadata::EMPTY; 16],
            vec![0u8; 2048],
        );
        let tx_buf = icmp::PacketBuffer::new(
            vec![icmp::PacketMetadata::EMPTY; 16],
            vec![0u8; 2048],
        );
        let icmp_sock = icmp::Socket::new(rx_buf, tx_buf);
        let ping_socket = sockets.add(icmp_sock);

        Self {
            id: id.into(),
            name: name.into(),
            mac,
            ip_cidr: None,
            gateway: None,
            device,
            iface,
            sockets,
            ping_socket,
            pending_pings: Vec::new(),
            ping_replies: Vec::new(),
        }
    }

    /// Configure IPv4 address/prefix and optional default gateway.
    pub fn configure_ip(&mut self, ip: Option<Ipv4Addr>, prefix_len: u8, gateway: Option<Ipv4Addr>) {
        self.ip_cidr = ip.map(|addr| (addr, prefix_len));
        self.gateway = gateway;

        self.iface.update_ip_addrs(|addrs| {
            addrs.clear();
            if let Some(addr) = ip {
                let octets = addr.octets();
                let cidr = Ipv4Cidr::new(Ipv4Address::from_octets(octets), prefix_len);
                let _ = addrs.push(IpCidr::Ipv4(cidr));
            }
        });

        self.iface.routes_mut().remove_default_ipv4_route();
        if let Some(gw) = gateway {
            let gw_octets = gw.octets();
            let _ = self
                .iface
                .routes_mut()
                .add_default_ipv4_route(Ipv4Address::from_octets(gw_octets));
        }
    }

    /// Enqueue an ICMP Echo Request to `target`.
    pub fn send_ping(&mut self, target: Ipv4Addr, ident: u16, seq: u16, now_ms: u64) -> Result<(), String> {
        let socket = self.sockets.get_mut::<icmp::Socket>(self.ping_socket);
        if !socket.is_open() {
            socket
                .bind(icmp::Endpoint::Ident(ident))
                .map_err(|e| format!("Failed to bind ICMP socket: {e:?}"))?;
        }

        let payload = b"netlab-ping-payload";
        let repr = Icmpv4Repr::EchoRequest {
            ident,
            seq_no: seq,
            data: payload,
        };

        let dst = IpAddress::Ipv4(Ipv4Address::from_octets(target.octets()));
        let buf = socket
            .send(repr.buffer_len(), dst)
            .map_err(|e| format!("ICMP send buffer error: {e:?}"))?;
        let mut pkt = Icmpv4Packet::new_unchecked(buf);
        repr.emit(&mut pkt, &ChecksumCapabilities::default());

        self.pending_pings.push(PendingPing {
            target,
            ident,
            seq,
            sent_at_ms: now_ms,
        });
        Ok(())
    }

    /// Poll `smoltcp` interface and process incoming ICMP Echo Replies.
    pub fn poll(&mut self, now_ms: u64) {
        let timestamp = Instant::from_millis(now_ms as i64);
        self.iface
            .poll(timestamp, &mut self.device, &mut self.sockets);

        let socket = self.sockets.get_mut::<icmp::Socket>(self.ping_socket);
        while socket.can_recv() {
            if let Ok((data, addr)) = socket.recv() {
                if let Ok(pkt) = Icmpv4Packet::new_checked(data) {
                    if let Ok(Icmpv4Repr::EchoReply { ident, seq_no, .. }) =
                        Icmpv4Repr::parse(&pkt, &ChecksumCapabilities::default())
                    {
                        if let IpAddress::Ipv4(v4) = addr {
                            self.ping_replies.push(PingReply {
                                from: Ipv4Addr::from(v4.octets()),
                                ident,
                                seq: seq_no,
                                received_at_ms: now_ms,
                            });
                        }
                    }
                }
            }
        }
    }
}
