use std::net::Ipv4Addr;

use net_sim_core::{Endpoint, FirewallAction, FirewallProtocol, NetworkWorld};
use serde_json::json;

#[test]
fn firewall_filters_icmp_and_tcp_ports() {
    let mut world = NetworkWorld::new();
    world.add_host("h1", "Client");
    world.add_host("h2", "Web Server");
    world.add_firewall("fw1", "Perimeter FW");

    world
        .configure_host_ip(
            "h1",
            Some(Ipv4Addr::new(10, 0, 1, 10)),
            24,
            Some(Ipv4Addr::new(10, 0, 1, 1)),
        )
        .unwrap();
    world
        .configure_host_ip(
            "h2",
            Some(Ipv4Addr::new(10, 0, 2, 10)),
            24,
            Some(Ipv4Addr::new(10, 0, 2, 1)),
        )
        .unwrap();

    world
        .connect("c1", Endpoint::new("h1", "eth0"), Endpoint::new("fw1", "eth0"))
        .unwrap();
    world
        .connect("c2", Endpoint::new("fw1", "eth1"), Endpoint::new("h2", "eth0"))
        .unwrap();

    world
        .configure_firewall_interface("fw1", "eth0", Some(Ipv4Addr::new(10, 0, 1, 1)), 24)
        .unwrap();
    world
        .configure_firewall_interface("fw1", "eth1", Some(Ipv4Addr::new(10, 0, 2, 1)), 24)
        .unwrap();

    // 1. Default policy Allow -> Ping succeeds
    world
        .send_ping("h1", Ipv4Addr::new(10, 0, 2, 10), 0x1234, 1)
        .unwrap();
    world.step(100);
    assert_eq!(world.hosts.get("h1").unwrap().ping_replies.len(), 1);

    // 2. Add Deny ICMP rule -> Ping blocked
    let rule_id = world
        .add_firewall_rule(
            "fw1",
            FirewallAction::Deny,
            FirewallProtocol::ICMP,
            Some("10.0.1.0/24".into()),
            None,
            None,
        )
        .unwrap();

    world
        .send_ping("h1", Ipv4Addr::new(10, 0, 2, 10), 0x1234, 2)
        .unwrap();
    world.step(100);
    assert_eq!(world.hosts.get("h1").unwrap().ping_replies.len(), 1);
    let fw = world.firewalls.get("fw1").unwrap();
    assert!(fw.packets_dropped >= 1);
    assert!(fw.rules.iter().find(|r| r.id == rule_id).unwrap().hits >= 1);

    // 3. HTTP on TCP/80 still allowed!
    world.attach_plugin("h2", "http-server").unwrap();
    world.attach_plugin("h1", "http-client").unwrap();
    world
        .run_plugin_command(
            "h1",
            "http-client",
            json!({ "command": "request", "method": "GET", "url": "http://10.0.2.10/" }),
        )
        .unwrap();
    world.step(200);
    let http_state = world
        .hosts
        .get("h1")
        .unwrap()
        .plugins
        .iter()
        .find(|p| p.id() == "http-client")
        .unwrap()
        .get_state();
    assert_eq!(
        http_state["last_response"]["status"].as_str(),
        Some("HTTP/1.1 200 OK")
    );
}
