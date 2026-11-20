use std::net::Ipv4Addr;

use serde_json::{json, Value};
use smoltcp::iface::SocketHandle;
use smoltcp::socket::tcp;
use smoltcp::wire::{IpAddress, IpEndpoint, Ipv4Address};

use crate::plugin::{
    AppPlugin, ConfigFieldSchema, PluginActionSchema, PluginContext, PluginMetadata,
};

pub struct FileServerPlugin {
    handle: Option<SocketHandle>,
    port: u16,
    transfers: u64,
}

impl FileServerPlugin {
    pub fn new() -> Self {
        Self {
            handle: None,
            port: 2121,
            transfers: 0,
        }
    }

    pub fn metadata() -> PluginMetadata {
        PluginMetadata {
            id: "file-server".into(),
            name: "File Server (FTP-Lite)".into(),
            description: "Serves and stores files from this node's virtual filesystem on TCP/2121."
                .into(),
            config_schema: vec![ConfigFieldSchema {
                key: "port".into(),
                label: "Listen Port".into(),
                field_type: "number".into(),
                default_value: json!(2121),
            }],
            actions: vec![PluginActionSchema {
                command: "write_file".into(),
                label: "Create Local File".into(),
                fields: vec![
                    ConfigFieldSchema {
                        key: "filename".into(),
                        label: "Filename".into(),
                        field_type: "text".into(),
                        default_value: json!("readme.txt"),
                    },
                    ConfigFieldSchema {
                        key: "content".into(),
                        label: "Content".into(),
                        field_type: "textarea".into(),
                        default_value: json!("Hello from NetLab File Server!"),
                    },
                ],
            }],
        }
    }
}

impl AppPlugin for FileServerPlugin {
    fn id(&self) -> &'static str {
        "file-server"
    }

    fn name(&self) -> &'static str {
        "File Server (FTP-Lite)"
    }

    fn on_mount(&mut self, ctx: &mut PluginContext) {
        if !ctx.files.contains_key("readme.txt") {
            ctx.files
                .insert("readme.txt".into(), "NetLab File Server v1.0\n".into());
        }
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
        if !sock.is_open() {
            let _ = sock.listen(self.port);
        }
        if sock.can_recv() {
            let mut req = Vec::new();
            let _ = sock.recv(|buf| {
                req.extend_from_slice(buf);
                (buf.len(), ())
            });
            let text = String::from_utf8_lossy(&req);
            let mut lines = text.splitn(2, '\n');
            let header = lines.next().unwrap_or("").trim();
            let rest = lines.next().unwrap_or("");
            let parts: Vec<&str> = header.split_whitespace().collect();
            let reply = match parts.as_slice() {
                ["LIST"] => {
                    let list: Vec<String> = ctx.files.keys().cloned().collect();
                    format!("200 OK\n{}", list.join("\n"))
                }
                ["GET", name] => {
                    if let Some(content) = ctx.files.get(*name) {
                        self.transfers += 1;
                        format!("200 OK\n{content}")
                    } else {
                        "404 Not Found\n".to_string()
                    }
                }
                ["PUT", name] => {
                    ctx.files.insert((*name).to_string(), rest.to_string());
                    self.transfers += 1;
                    ctx.console_logs
                        .push(format!("[file-server] Stored {name} ({} bytes)", rest.len()));
                    format!("201 Stored {name}\n")
                }
                _ => "400 Bad Command\n".to_string(),
            };
            let _ = sock.send_slice(reply.as_bytes());
            sock.close();
        } else if sock.state() == tcp::State::CloseWait {
            sock.close();
        }
    }

    fn handle_command(&mut self, cmd: Value, ctx: &mut PluginContext) -> Result<Value, String> {
        let action = cmd.get("command").and_then(|v| v.as_str()).unwrap_or("");
        match action {
            "configure" => {
                let cfg = cmd.get("config").unwrap_or(&cmd);
                if let Some(p) = cfg.get("port").and_then(|v| v.as_u64()) {
                    self.port = p as u16;
                }
                Ok(json!({ "ok": true }))
            }
            "write_file" => {
                let filename = cmd
                    .get("filename")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing filename")?;
                let content = cmd.get("content").and_then(|v| v.as_str()).unwrap_or("");
                ctx.files.insert(filename.to_string(), content.to_string());
                Ok(json!({ "ok": true }))
            }
            _ => Err(format!("Unknown file-server command: {action}")),
        }
    }

    fn get_state(&self) -> Value {
        json!({
            "port": self.port,
            "transfers": self.transfers,
        })
    }
}

pub struct FileClientPlugin {
    handle: Option<SocketHandle>,
    local_port: u16,
    pending_op: Option<(String, String, String)>, // (op, filename, payload)
    sent: bool,
    rx_buf: Vec<u8>,
    last_status: Option<String>,
}

impl FileClientPlugin {
    pub fn new() -> Self {
        Self {
            handle: None,
            local_port: 50100,
            pending_op: None,
            sent: false,
            rx_buf: Vec::new(),
            last_status: None,
        }
    }

    pub fn metadata() -> PluginMetadata {
        PluginMetadata {
            id: "file-client".into(),
            name: "File Client (FTP-Lite)".into(),
            description: "Downloads or uploads files to a remote File Server on TCP/2121.".into(),
            config_schema: vec![],
            actions: vec![
                PluginActionSchema {
                    command: "download".into(),
                    label: "Download File".into(),
                    fields: vec![
                        ConfigFieldSchema {
                            key: "server".into(),
                            label: "Server IP or Hostname".into(),
                            field_type: "text".into(),
                            default_value: json!("10.0.2.10"),
                        },
                        ConfigFieldSchema {
                            key: "filename".into(),
                            label: "Filename".into(),
                            field_type: "text".into(),
                            default_value: json!("readme.txt"),
                        },
                    ],
                },
                PluginActionSchema {
                    command: "upload".into(),
                    label: "Upload File".into(),
                    fields: vec![
                        ConfigFieldSchema {
                            key: "server".into(),
                            label: "Server IP or Hostname".into(),
                            field_type: "text".into(),
                            default_value: json!("10.0.2.10"),
                        },
                        ConfigFieldSchema {
                            key: "filename".into(),
                            label: "Filename".into(),
                            field_type: "text".into(),
                            default_value: json!("note.txt"),
                        },
                        ConfigFieldSchema {
                            key: "content".into(),
                            label: "Content".into(),
                            field_type: "textarea".into(),
                            default_value: json!("Uploaded from client"),
                        },
                    ],
                },
            ],
        }
    }
}

impl AppPlugin for FileClientPlugin {
    fn id(&self) -> &'static str {
        "file-client"
    }

    fn name(&self) -> &'static str {
        "File Client (FTP-Lite)"
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

        if let Some((op, filename, payload)) = &self.pending_op {
            if sock.can_send() && !self.sent {
                let _ = sock.send_slice(payload.as_bytes());
                self.sent = true;
            }
            if sock.can_recv() {
                let _ = sock.recv(|buf| {
                    self.rx_buf.extend_from_slice(buf);
                    (buf.len(), ())
                });
            }
            if sock.state() == tcp::State::CloseWait || sock.state() == tcp::State::Closed {
                if !self.rx_buf.is_empty() {
                    let resp = String::from_utf8_lossy(&self.rx_buf).to_string();
                    let mut parts = resp.splitn(2, '\n');
                    let status = parts.next().unwrap_or("").trim().to_string();
                    let body = parts.next().unwrap_or("").to_string();
                    if op == "GET" && status.starts_with("200") {
                        ctx.files.insert(filename.clone(), body);
                        ctx.console_logs
                            .push(format!("[file-client] Downloaded {filename}"));
                    } else {
                        ctx.console_logs
                            .push(format!("[file-client] {op} {filename}: {status}"));
                    }
                    self.last_status = Some(status);
                }
                sock.close();
                self.pending_op = None;
                self.sent = false;
                self.rx_buf.clear();
            }
        }
    }

    fn handle_command(&mut self, cmd: Value, ctx: &mut PluginContext) -> Result<Value, String> {
        let action = cmd.get("command").and_then(|v| v.as_str()).unwrap_or("");
        let server = cmd
            .get("server")
            .and_then(|v| v.as_str())
            .ok_or("Missing server")?
            .trim();
        let filename = cmd
            .get("filename")
            .and_then(|v| v.as_str())
            .ok_or("Missing filename")?
            .trim();

        let target_ip: Ipv4Addr = if let Ok(ip) = server.parse() {
            ip
        } else if let Some(&cached) = ctx.dns_cache.get(&server.to_lowercase()) {
            cached
        } else {
            return Err(format!("Host '{server}' not in DNS cache"));
        };

        let (op, payload) = match action {
            "download" => ("GET".to_string(), format!("GET {filename}\n")),
            "upload" => {
                let content = cmd.get("content").and_then(|v| v.as_str()).unwrap_or("");
                ("PUT".to_string(), format!("PUT {filename}\n{content}"))
            }
            _ => return Err(format!("Unknown file-client command: {action}")),
        };

        let h = self.handle.ok_or("File client socket not mounted")?;
        let sock = ctx.sockets.get_mut::<tcp::Socket>(h);
        sock.abort();
        self.local_port = self.local_port.wrapping_add(1).max(50100);
        let remote = IpEndpoint::new(
            IpAddress::Ipv4(Ipv4Address::from_octets(target_ip.octets())),
            2121,
        );
        sock.connect(ctx.iface.context(), remote, self.local_port)
            .map_err(|e| format!("TCP connect failed: {e:?}"))?;

        self.pending_op = Some((op, filename.to_string(), payload));
        self.sent = false;
        self.rx_buf.clear();
        Ok(json!({ "status": "connecting" }))
    }

    fn get_state(&self) -> Value {
        json!({ "last_status": self.last_status })
    }
}
