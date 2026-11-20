use std::net::Ipv4Addr;

use net_sim_core::{Endpoint, NetworkWorld};

#[test]
fn packet_tap_records_and_exports_valid_pcap() {
    let mut world = NetworkWorld::new();
    world.add_host("h1", "Host A");
    world.add_host("h2", "Host B");

    world
        .configure_host_ip("h1", Some(Ipv4Addr::new(10, 0, 0, 1)), 24, None)
        .unwrap();
    world
        .configure_host_ip("h2", Some(Ipv4Addr::new(10, 0, 0, 2)), 24, None)
        .unwrap();
    world
        .connect("c1", Endpoint::new("h1", "eth0"), Endpoint::new("h2", "eth0"))
        .unwrap();

    world
        .send_ping("h1", Ipv4Addr::new(10, 0, 0, 2), 0x1234, 1)
        .unwrap();
    world.step(50);

    let packets = world.drain_packets();
    assert!(packets.len() >= 4, "Expected ARP req/rep + ICMP req/rep");

    let protocols: Vec<&str> = packets.iter().map(|p| p.summary.protocol.as_str()).collect();
    assert!(protocols.contains(&"ARP"));
    assert!(protocols.contains(&"ICMP"));

    let icmp_pkt = packets.iter().find(|p| p.summary.protocol == "ICMP").unwrap();
    assert_eq!(icmp_pkt.summary.src_ip.as_deref(), Some("10.0.0.1"));
    assert_eq!(icmp_pkt.summary.dst_ip.as_deref(), Some("10.0.0.2"));
    assert!(icmp_pkt.summary.layers.len() >= 3);

    let pcap = world.export_pcap();
    // Check Libpcap magic number 0xa1b2c3d4 in little-endian
    assert!(pcap.len() > 24);
    assert_eq!(&pcap[0..4], &0xa1b2c3d4u32.to_le_bytes());
    // Linktype Ethernet = 1 at bytes 20..24
    assert_eq!(&pcap[20..24], &1u32.to_le_bytes());
}
