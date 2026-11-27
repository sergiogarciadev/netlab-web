use std::collections::BTreeMap;
use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use smoltcp::iface::SocketHandle;
use smoltcp::socket::tcp;
use smoltcp::wire::{IpAddress, IpEndpoint, Ipv4Address};

use crate::plugin::{
    AppPlugin, ConfigFieldSchema, PluginActionSchema, PluginContext, PluginMetadata,
};

pub struct HttpServerPlugin {
    handles: Vec<SocketHandle>,
    port: u16,
    routes: BTreeMap<String, String>,
    requests_served: u64,
}

impl HttpServerPlugin {
    pub fn new() -> Self {
        let mut routes = BTreeMap::new();
        routes.insert("/".into(), "<h1>Welcome to NetLab HTTP Server</h1>".into());
        routes.insert("/api/status".into(), "{\"status\":\"ok\",\"service\":\"netlab\"}".into());
        Self {
            handles: Vec::new(),
            port: 80,
            routes,
            requests_served: 0,
        }
    }

    pub fn metadata() -> PluginMetadata {
        PluginMetadata {
            id: "http-server".into(),
            name: "HTTP Server".into(),
            description: "Serves static HTML/JSON routes and accepts POST requests on TCP/80."
                .into(),
            config_schema: vec![
                ConfigFieldSchema {
                    key: "port".into(),
                    label: "Listen Port".into(),
                    field_type: "number".into(),
                    default_value: json!(80),
                },
                ConfigFieldSchema {
                    key: "routes_json".into(),
                    label: "Routes (JSON path -> body)".into(),
                    field_type: "textarea".into(),
                    default_value: json!("{\"/\": \"<h1>Welcome to NetLab</h1>\", \"/api/status\": \"{\\\"status\\\":\\\"ok\\\"}\"}"),
                },
            ],
            actions: vec![],
        }
    }

    fn ensure_sockets(&mut self, ctx: &mut PluginContext) {
        if self.handles.is_empty() {
            for _ in 0..3 {
                let sock = tcp::Socket::new(
                    tcp::SocketBuffer::new(vec![0u8; 4096]),
                    tcp::SocketBuffer::new(vec![0u8; 4096]),
                );
                self.handles.push(ctx.sockets.add(sock));
            }
        }
    }
}

impl AppPlugin for HttpServerPlugin {
    fn id(&self) -> &'static str {
        "http-server"
    }

    fn name(&self) -> &'static str {
        "HTTP Server"
    }

    fn on_mount(&mut self, ctx: &mut PluginContext) {
        self.ensure_sockets(ctx);
    }

    fn on_unmount(&mut self, ctx: &mut PluginContext) {
        for h in self.handles.drain(..) {
            ctx.sockets.remove(h);
        }
    }

    fn poll(&mut self, ctx: &mut PluginContext) {
        self.ensure_sockets(ctx);
        for &h in &self.handles {
            let sock = ctx.sockets.get_mut::<tcp::Socket>(h);
            if !sock.is_open() {
                let _ = sock.listen(self.port);
            }
            if sock.can_recv() {
                let mut req_bytes = Vec::new();
                let _ = sock.recv(|buf| {
                    req_bytes.extend_from_slice(buf);
                    (buf.len(), ())
                });
                let req_str = String::from_utf8_lossy(&req_bytes);
                if let Some(first_line) = req_str.lines().next() {
                    let parts: Vec<&str> = first_line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let method = parts[0];
                        let path = parts[1];
                        let (status, body) = if method == "POST" {
                            let body_part = req_str
                                .split("\r\n\r\n")
                                .nth(1)
                                .unwrap_or("")
                                .to_string();
                            self.routes.insert(path.to_string(), body_part.clone());
                            ("201 Created", format!("{{\"stored\":\"{path}\"}}"))
                        } else if let Some(found) = self.routes.get(path) {
                            ("200 OK", found.clone())
                        } else {
                            ("404 Not Found", "Not Found".to_string())
                        };
                        let content_type = if body.trim_start().starts_with('{') {
                            "application/json"
                        } else {
                            "text/html"
                        };
                        let resp = format!(
                            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                            body.len()
                        );
                        let _ = sock.send_slice(resp.as_bytes());
                        sock.close();
                        self.requests_served += 1;
                        ctx.console_logs
                            .push(format!("[http-server] {method} {path} -> {status}"));
                    }
                }
            } else if sock.state() == tcp::State::CloseWait {
                sock.close();
            }
        }
    }

    fn handle_command(&mut self, cmd: Value, _ctx: &mut PluginContext) -> Result<Value, String> {
        let action = cmd.get("command").and_then(|v| v.as_str()).unwrap_or("");
        if action == "configure" {
            let cfg = cmd.get("config").unwrap_or(&cmd);
            if let Some(p) = cfg.get("port").and_then(|v| v.as_u64()) {
                self.port = p as u16;
            }
            if let Some(rj) = cfg.get("routes_json").and_then(|v| v.as_str()) {
                let parsed: BTreeMap<String, String> =
                    serde_json::from_str(rj).map_err(|e| format!("Invalid routes JSON: {e}"))?;
                self.routes = parsed;
            }
            return Ok(json!({ "ok": true }));
        }
        Err(format!("Unknown http-server command: {action}"))
    }

    fn get_state(&self) -> Value {
        json!({
            "port": self.port,
            "routes_json": serde_json::to_string_pretty(&self.routes).unwrap_or_default(),
            "requests_served": self.requests_served,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponseRecord {
    pub url: String,
    pub status: String,
    pub body: String,
}

pub struct HttpClientPlugin {
    handle: Option<SocketHandle>,
    local_port: u16,
    pending_request: Option<(String, String)>, // (url, raw_http_request)
    sent_request: bool,
    rx_buf: Vec<u8>,
    last_response: Option<HttpResponseRecord>,
}

impl HttpClientPlugin {
    pub fn new() -> Self {
        Self {
            handle: None,
            local_port: 49152,
            pending_request: None,
            sent_request: false,
            rx_buf: Vec::new(),
            last_response: None,
        }
    }

    pub fn metadata() -> PluginMetadata {
        PluginMetadata {
            id: "http-client".into(),
            name: "HTTP Client".into(),
            description: "Sends HTTP GET/POST requests to IP addresses or DNS-resolved hostnames."
                .into(),
            config_schema: vec![],
            actions: vec![PluginActionSchema {
                command: "request".into(),
                label: "Send HTTP Request".into(),
                fields: vec![
                    ConfigFieldSchema {
                        key: "method".into(),
                        label: "Method (GET/POST)".into(),
                        field_type: "text".into(),
                        default_value: json!("GET"),
                    },
                    ConfigFieldSchema {
                        key: "url".into(),
                        label: "URL (e.g. http://web.local/)".into(),
                        field_type: "text".into(),
                        default_value: json!("http://web.local/"),
                    },
                    ConfigFieldSchema {
                        key: "body".into(),
                        label: "Body (for POST)".into(),
                        field_type: "text".into(),
                        default_value: json!(""),
                    },
                ],
            }],
        }
    }
}

impl AppPlugin for HttpClientPlugin {
    fn id(&self) -> &'static str {
        "http-client"
    }

    fn name(&self) -> &'static str {
        "HTTP Client"
    }

    fn on_mount(&mut self, ctx: &mut PluginContext) {
        let sock = tcp::Socket::new(
            tcp::SocketBuffer::new(vec![0u8; 4096]),
            tcp::SocketBuffer::new(vec![0u8; 4096]),
        );
        self.handle = Some(ctx.sockets.add(sock));
    }

    fn on_unmount(&mut self, ctx: &mut PluginContext) {
        if let Some(h) = self.handle.take() {
            ctx.sockets.remove(h);
        }
    }

    fn poll(&mut self, ctx: &mut PluginContext) {
        let Some(h) = self.handle else { return };
        let sock = ctx.sockets.get_mut::<tcp::Socket>(h);

        if let Some((url, raw_req)) = &self.pending_request {
            if sock.can_send() && !self.sent_request {
                let _ = sock.send_slice(raw_req.as_bytes());
                self.sent_request = true;
            }
            if sock.can_recv() {
                let _ = sock.recv(|buf| {
                    self.rx_buf.extend_from_slice(buf);
                    (buf.len(), ())
                });
            }
            if sock.state() == tcp::State::CloseWait || sock.state() == tcp::State::Closed {
                if !self.rx_buf.is_empty() {
                    let full = String::from_utf8_lossy(&self.rx_buf).to_string();
                    let status = full.lines().next().unwrap_or("HTTP/1.1").to_string();
                    let body = full.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
                    let rec = HttpResponseRecord {
                        url: url.clone(),
                        status: status.clone(),
                        body: body.clone(),
                    };
                    ctx.console_logs
                        .push(format!("[http-client] {url} -> {status} ({} bytes)", body.len()));
                    self.last_response = Some(rec);
                }
                sock.close();
                self.pending_request = None;
                self.sent_request = false;
                self.rx_buf.clear();
            }
        }
    }

    fn handle_command(&mut self, cmd: Value, ctx: &mut PluginContext) -> Result<Value, String> {
        let action = cmd.get("command").and_then(|v| v.as_str()).unwrap_or("");
        if action == "configure" {
            return Ok(json!({ "ok": true }));
        }
        if action != "request" {
            return Err(format!("Unknown http-client command: {action}"));
        }
        let method = cmd
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_uppercase();
        let url = cmd
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or("Missing url")?
            .trim()
            .to_string();
        let body = cmd.get("body").and_then(|v| v.as_str()).unwrap_or("");

        let stripped = url.strip_prefix("http://").unwrap_or(&url);
        let (host_port, path) = match stripped.find('/') {
            Some(idx) => (&stripped[..idx], &stripped[idx..]),
            None => (stripped, "/"),
        };
        let (host_str, port) = match host_port.split_once(':') {
            Some((h, p)) => (h, p.parse::<u16>().unwrap_or(80)),
            None => (host_port, 80),
        };

        let target_ip: Ipv4Addr = if let Ok(ip) = host_str.parse::<Ipv4Addr>() {
            ip
        } else if let Some(&cached) = ctx.dns_cache.get(&host_str.to_lowercase()) {
            cached
        } else {
            return Err(format!(
                "Host '{host_str}' not in DNS cache. Run DNS lookup first."
            ));
        };

        let raw_req = if method == "POST" {
            format!(
                "POST {path} HTTP/1.1\r\nHost: {host_str}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
        } else {
            format!("GET {path} HTTP/1.1\r\nHost: {host_str}\r\nConnection: close\r\n\r\n")
        };

        let h = self.handle.ok_or("HTTP client socket not mounted")?;
        let sock = ctx.sockets.get_mut::<tcp::Socket>(h);
        sock.abort();
        self.local_port = self.local_port.wrapping_add(1).max(49152);
        let remote = IpEndpoint::new(
            IpAddress::Ipv4(Ipv4Address::from_octets(target_ip.octets())),
            port,
        );
        sock.connect(ctx.iface.context(), remote, self.local_port)
            .map_err(|e| format!("TCP connect failed: {e:?}"))?;

        self.pending_request = Some((url, raw_req));
        self.sent_request = false;
        self.rx_buf.clear();

        Ok(json!({ "status": "connecting", "target_ip": target_ip.to_string(), "port": port }))
    }

    fn get_state(&self) -> Value {
        json!({
            "last_response": self.last_response,
        })
    }
}
