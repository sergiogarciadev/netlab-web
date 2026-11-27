use std::net::Ipv4Addr;

use net_sim_core::{CableConfig, Endpoint, NetworkWorld};

#[test]
fn host_a_pings_host_b_directly_over_cable() {
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
        .connect(
            "c1",
            Endpoint::new("h1", "eth0"),
            Endpoint::new("h2", "eth0"),
        )
        .unwrap();

    world
        .send_ping("h1", Ipv4Addr::new(10, 0, 0, 2), 0x1234, 1)
        .unwrap();

    // Advance enough time for ARP + ICMP Echo Request + ICMP Echo Reply
    world.step(100);
    eprintln!("Events: {:?}", world.drain_animation_events());

    let h1 = world.hosts.get("h1").unwrap();
    assert_eq!(h1.ping_replies.len(), 1, "Expected 1 ping reply on h1");
    assert_eq!(h1.ping_replies[0].from, Ipv4Addr::new(10, 0, 0, 2));
    assert_eq!(h1.ping_replies[0].seq, 1);
}

#[test]
fn host_a_pings_host_b_across_virtual_switch() {
    let mut world = NetworkWorld::new();
    world.add_host("h1", "Host A");
    world.add_host("h2", "Host B");
    world.add_host("h3", "Host C");
    world.add_switch("sw1", "Switch 1");

    world
        .configure_host_ip("h1", Some(Ipv4Addr::new(192, 168, 1, 10)), 24, None)
        .unwrap();
    world
        .configure_host_ip("h2", Some(Ipv4Addr::new(192, 168, 1, 20)), 24, None)
        .unwrap();
    world
        .configure_host_ip("h3", Some(Ipv4Addr::new(192, 168, 1, 30)), 24, None)
        .unwrap();

    world
        .connect(
            "c1",
            Endpoint::new("h1", "eth0"),
            Endpoint::new("sw1", "p1"),
        )
        .unwrap();
    world
        .connect(
            "c2",
            Endpoint::new("h2", "eth0"),
            Endpoint::new("sw1", "p2"),
        )
        .unwrap();
    world
        .connect(
            "c3",
            Endpoint::new("h3", "eth0"),
            Endpoint::new("sw1", "p3"),
        )
        .unwrap();

    world
        .send_ping("h1", Ipv4Addr::new(192, 168, 1, 20), 0x1234, 1)
        .unwrap();

    world.step(150);

    let h1 = world.hosts.get("h1").unwrap();
    assert_eq!(h1.ping_replies.len(), 1);
    assert_eq!(h1.ping_replies[0].from, Ipv4Addr::new(192, 168, 1, 20));

    // Verify switch learned both h1 and h2 MAC addresses
    let sw = world.switches.get("sw1").unwrap();
    let h1_mac = world.hosts.get("h1").unwrap().mac;
    let h2_mac = world.hosts.get("h2").unwrap().mac;
    assert_eq!(sw.mac_table.get(&h1_mac).map(|e| e.port_id.as_str()), Some("p1"));
    assert_eq!(sw.mac_table.get(&h2_mac).map(|e| e.port_id.as_str()), Some("p2"));
}

#[test]
fn host_a_subnet1_pings_host_b_subnet2_across_virtual_router() {
    let mut world = NetworkWorld::new();
    world.add_host("h1", "Host A");
    world.add_host("h2", "Host B");
    world.add_router("r1", "Router 1");

    // Subnet 1: 10.0.1.0/24, gateway 10.0.1.1
    world
        .configure_host_ip(
            "h1",
            Some(Ipv4Addr::new(10, 0, 1, 10)),
            24,
            Some(Ipv4Addr::new(10, 0, 1, 1)),
        )
        .unwrap();
    // Subnet 2: 10.0.2.0/24, gateway 10.0.2.1
    world
        .configure_host_ip(
            "h2",
            Some(Ipv4Addr::new(10, 0, 2, 20)),
            24,
            Some(Ipv4Addr::new(10, 0, 2, 1)),
        )
        .unwrap();

    world
        .connect(
            "c1",
            Endpoint::new("h1", "eth0"),
            Endpoint::new("r1", "eth0"),
        )
        .unwrap();
    world
        .connect(
            "c2",
            Endpoint::new("h2", "eth0"),
            Endpoint::new("r1", "eth1"),
        )
        .unwrap();

    world
        .configure_router_interface("r1", "eth0", Some(Ipv4Addr::new(10, 0, 1, 1)), 24)
        .unwrap();
    world
        .configure_router_interface("r1", "eth1", Some(Ipv4Addr::new(10, 0, 2, 1)), 24)
        .unwrap();

    world
        .send_ping("h1", Ipv4Addr::new(10, 0, 2, 20), 0x1234, 1)
        .unwrap();

    world.step(200);

    let h1 = world.hosts.get("h1").unwrap();
    assert_eq!(h1.ping_replies.len(), 1, "Expected cross-subnet ping reply");
    assert_eq!(h1.ping_replies[0].from, Ipv4Addr::new(10, 0, 2, 20));
    assert_eq!(h1.ping_replies[0].seq, 1);

    // Also test pinging the router's own gateway interface
    world
        .send_ping("h1", Ipv4Addr::new(10, 0, 1, 1), 0x1234, 2)
        .unwrap();
    world.step(100);
    let h1 = world.hosts.get("h1").unwrap();
    assert_eq!(h1.ping_replies.len(), 2);
    assert_eq!(h1.ping_replies[1].from, Ipv4Addr::new(10, 0, 1, 1));
}

#[test]
fn cable_packet_loss_and_link_down() {
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
        .connect(
            "c1",
            Endpoint::new("h1", "eth0"),
            Endpoint::new("h2", "eth0"),
        )
        .unwrap();

    // 100% packet loss -> no replies
    world
        .update_cable_config(
            "c1",
            CableConfig {
                latency_ms: 5,
                jitter_ms: 0,
                loss_percent: 100.0,
                up: true,
            },
        )
        .unwrap();

    world
        .send_ping("h1", Ipv4Addr::new(10, 0, 0, 2), 0x1234, 1)
        .unwrap();
    world.step(100);
    assert_eq!(world.hosts.get("h1").unwrap().ping_replies.len(), 0);
    assert!(world.cables.get("c1").unwrap().packets_dropped > 0);

    // Link down -> no replies
    world
        .update_cable_config(
            "c1",
            CableConfig {
                latency_ms: 5,
                jitter_ms: 0,
                loss_percent: 0.0,
                up: false,
            },
        )
        .unwrap();
    world
        .send_ping("h1", Ipv4Addr::new(10, 0, 0, 2), 0x1234, 2)
        .unwrap();
    world.step(100);
    assert_eq!(world.hosts.get("h1").unwrap().ping_replies.len(), 0);

    // Link up with 0% loss -> ping succeeds
    world
        .update_cable_config(
            "c1",
            CableConfig {
                latency_ms: 5,
                jitter_ms: 0,
                loss_percent: 0.0,
                up: true,
            },
        )
        .unwrap();
    world.step(1100);
    world
        .send_ping("h1", Ipv4Addr::new(10, 0, 0, 2), 0x1234, 3)
        .unwrap();
    world.step(100);
    let replies = &world.hosts.get("h1").unwrap().ping_replies;
    assert!(!replies.is_empty(), "Expected ping replies after link recovery");
    assert_eq!(replies.last().unwrap().seq, 3);
}

#[test]
fn step_hop_advances_one_wire_at_a_time() {
    let mut world = NetworkWorld::new();
    world.add_host("h1", "Host A");
    world.add_host("h2", "Host B");
    world.add_switch("sw1", "Switch 1");

    world
        .configure_host_ip("h1", Some(Ipv4Addr::new(10, 0, 0, 1)), 24, None)
        .unwrap();
    world
        .configure_host_ip("h2", Some(Ipv4Addr::new(10, 0, 0, 2)), 24, None)
        .unwrap();
    world
        .connect("c1", Endpoint::new("h1", "eth0"), Endpoint::new("sw1", "p1"))
        .unwrap();
    world
        .connect("c2", Endpoint::new("sw1", "p2"), Endpoint::new("h2", "eth0"))
        .unwrap();

    world
        .send_ping("h1", Ipv4Addr::new(10, 0, 0, 2), 0x1234, 1)
        .unwrap();

    // Hop 1: h1 transmits ARP Request onto c1 ONLY (not c2 yet!)
    world.step_hop(2000);
    let ev1 = world.drain_animation_events();
    assert_eq!(ev1.len(), 1);
    assert_eq!(ev1[0].cable_id, "c1");
    assert_eq!(ev1[0].protocol, "ARP");

    // Hop 2: c1 delivers ARP Request to sw1, sw1 forwards onto c2
    world.step_hop(2000);
    let ev2 = world.drain_animation_events();
    assert_eq!(ev2.len(), 1);
    assert_eq!(ev2[0].cable_id, "c2");
    assert_eq!(ev2[0].protocol, "ARP");
}
