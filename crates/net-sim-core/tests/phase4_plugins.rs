use std::net::Ipv4Addr;

use net_sim_core::{Endpoint, NetworkWorld};
use serde_json::json;

#[test]
fn full_multi_subnet_dhcp_dns_http_file_mail_workflow() {
    let mut world = NetworkWorld::new();

    // Subnet 1: 10.0.1.0/24 (Client + DHCP Server + Router eth0)
    // Subnet 2: 10.0.2.0/24 (App Server: DNS + HTTP + File + Mail + Router eth1)
    world.add_host("client", "Client");
    world.add_host("dhcp", "DHCP Server");
    world.add_host("srv", "App Server");
    world.add_switch("sw1", "LAN Switch");
    world.add_router("r1", "Core Router");

    world
        .connect("c1", Endpoint::new("client", "eth0"), Endpoint::new("sw1", "p1"))
        .unwrap();
    world
        .connect("c2", Endpoint::new("dhcp", "eth0"), Endpoint::new("sw1", "p2"))
        .unwrap();
    world
        .connect("c3", Endpoint::new("r1", "eth0"), Endpoint::new("sw1", "p3"))
        .unwrap();
    world
        .connect("c4", Endpoint::new("r1", "eth1"), Endpoint::new("srv", "eth0"))
        .unwrap();

    world
        .configure_router_interface("r1", "eth0", Some(Ipv4Addr::new(10, 0, 1, 1)), 24)
        .unwrap();
    world
        .configure_router_interface("r1", "eth1", Some(Ipv4Addr::new(10, 0, 2, 1)), 24)
        .unwrap();

    world
        .configure_host_ip(
            "dhcp",
            Some(Ipv4Addr::new(10, 0, 1, 2)),
            24,
            Some(Ipv4Addr::new(10, 0, 1, 1)),
        )
        .unwrap();
    world
        .configure_host_ip(
            "srv",
            Some(Ipv4Addr::new(10, 0, 2, 10)),
            24,
            Some(Ipv4Addr::new(10, 0, 2, 1)),
        )
        .unwrap();

    // 1. Mount DHCP Server & Client
    world.attach_plugin("dhcp", "dhcp-server").unwrap();
    world
        .run_plugin_command(
            "dhcp",
            "dhcp-server",
            json!({
                "command": "configure",
                "config": {
                    "pool_start": "10.0.1.100",
                    "pool_end": "10.0.1.150",
                    "prefix_len": 24,
                    "gateway": "10.0.1.1",
                    "dns_server": "10.0.2.10"
                }
            }),
        )
        .unwrap();

    world.attach_plugin("client", "dhcp-client").unwrap();

    // Mount DNS, HTTP, File, Mail on srv
    world.attach_plugin("srv", "dns-server").unwrap();
    world
        .run_plugin_command(
            "srv",
            "dns-server",
            json!({
                "command": "configure",
                "config": {
                    "records_text": "web.local A 10.0.2.10\nmail.corp A 10.0.2.10\nfiles.corp CNAME web.local"
                }
            }),
        )
        .unwrap();
    world.attach_plugin("srv", "http-server").unwrap();
    world.attach_plugin("srv", "file-server").unwrap();
    world.attach_plugin("srv", "mail-server").unwrap();

    // Step simulation for DHCP lease
    world.step(300);

    let client = world.hosts.get("client").unwrap();
    assert_eq!(client.ip_cidr, Some((Ipv4Addr::new(10, 0, 1, 100), 24)));
    assert_eq!(client.gateway, Some(Ipv4Addr::new(10, 0, 1, 1)));
    assert_eq!(client.dns_servers, vec![Ipv4Addr::new(10, 0, 2, 10)]);

    // 2. Resolve web.local and mail.corp via DNS
    world.attach_plugin("client", "dns-resolver").unwrap();
    world
        .run_plugin_command(
            "client",
            "dns-resolver",
            json!({ "command": "lookup", "hostname": "web.local" }),
        )
        .unwrap();
    world
        .run_plugin_command(
            "client",
            "dns-resolver",
            json!({ "command": "lookup", "hostname": "mail.corp" }),
        )
        .unwrap();
    world.step(200);

    let client = world.hosts.get("client").unwrap();
    assert_eq!(
        client.dns_cache.get("web.local"),
        Some(&Ipv4Addr::new(10, 0, 2, 10))
    );
    assert_eq!(
        client.dns_cache.get("mail.corp"),
        Some(&Ipv4Addr::new(10, 0, 2, 10))
    );

    // 3. Fetch HTTP page
    world.attach_plugin("client", "http-client").unwrap();
    world
        .run_plugin_command(
            "client",
            "http-client",
            json!({ "command": "request", "method": "GET", "url": "http://web.local/" }),
        )
        .unwrap();
    world.step(200);

    let http_state = world
        .hosts
        .get("client")
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

    // 4. Download file via File Client
    world.attach_plugin("client", "file-client").unwrap();
    world
        .run_plugin_command(
            "client",
            "file-client",
            json!({ "command": "download", "server": "web.local", "filename": "readme.txt" }),
        )
        .unwrap();
    world.step(200);
    assert!(world
        .hosts
        .get("client")
        .unwrap()
        .files
        .contains_key("readme.txt"));

    // 5. Send email via Mail Client & fetch inbox
    world.attach_plugin("client", "mail-client").unwrap();
    world
        .run_plugin_command(
            "client",
            "mail-client",
            json!({
                "command": "send",
                "server": "mail.corp",
                "from": "alice@corp",
                "to": "bob@corp",
                "subject": "NetLab Test",
                "body": "Hello Bob!"
            }),
        )
        .unwrap();
    world.step(200);

    world
        .run_plugin_command(
            "client",
            "mail-client",
            json!({
                "command": "fetch",
                "server": "mail.corp",
                "user": "bob@corp"
            }),
        )
        .unwrap();
    world.step(200);

    let mail_state = world
        .hosts
        .get("client")
        .unwrap()
        .plugins
        .iter()
        .find(|p| p.id() == "mail-client")
        .unwrap()
        .get_state();
    assert_eq!(mail_state["inbox"].as_array().unwrap().len(), 1);
    assert_eq!(
        mail_state["inbox"][0]["subject"].as_str(),
        Some("NetLab Test")
    );
}
