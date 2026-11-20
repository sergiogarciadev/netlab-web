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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMessage {
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
    pub timestamp_ms: u64,
}

pub struct MailServerPlugin {
    smtp_handle: Option<SocketHandle>,
    pop3_handle: Option<SocketHandle>,
    domain: String,
    mailboxes: BTreeMap<String, Vec<EmailMessage>>,
}

impl MailServerPlugin {
    pub fn new() -> Self {
        Self {
            smtp_handle: None,
            pop3_handle: None,
            domain: "corp".into(),
            mailboxes: BTreeMap::new(),
        }
    }

    pub fn metadata() -> PluginMetadata {
        PluginMetadata {
            id: "mail-server".into(),
            name: "Mail Server (SMTP/POP3 Lite)".into(),
            description: "Receives emails on TCP/25 and serves mailboxes on TCP/110.".into(),
            config_schema: vec![ConfigFieldSchema {
                key: "domain".into(),
                label: "Mail Domain".into(),
                field_type: "text".into(),
                default_value: json!("corp"),
            }],
            actions: vec![],
        }
    }
}

impl AppPlugin for MailServerPlugin {
    fn id(&self) -> &'static str {
        "mail-server"
    }

    fn name(&self) -> &'static str {
        "Mail Server (SMTP/POP3 Lite)"
    }

    fn on_mount(&mut self, ctx: &mut PluginContext) {
        let smtp = tcp::Socket::new(
            tcp::SocketBuffer::new(vec![0u8; 4096]),
            tcp::SocketBuffer::new(vec![0u8; 4096]),
        );
        let pop3 = tcp::Socket::new(
            tcp::SocketBuffer::new(vec![0u8; 4096]),
            tcp::SocketBuffer::new(vec![0u8; 4096]),
        );
        self.smtp_handle = Some(ctx.sockets.add(smtp));
        self.pop3_handle = Some(ctx.sockets.add(pop3));
    }

    fn on_unmount(&mut self, ctx: &mut PluginContext) {
        if let Some(h) = self.smtp_handle.take() {
            ctx.sockets.remove(h);
        }
        if let Some(h) = self.pop3_handle.take() {
            ctx.sockets.remove(h);
        }
    }

    fn poll(&mut self, ctx: &mut PluginContext) {
        if let Some(h) = self.smtp_handle {
            let sock = ctx.sockets.get_mut::<tcp::Socket>(h);
            if !sock.is_open() {
                let _ = sock.listen(25);
            }
            if sock.can_recv() {
                let mut raw = Vec::new();
                let _ = sock.recv(|buf| {
                    raw.extend_from_slice(buf);
                    (buf.len(), ())
                });
                let text = String::from_utf8_lossy(&raw);
                if let Ok(msg) = serde_json::from_str::<EmailMessage>(text.trim()) {
                    let recipient = msg.to.clone();
                    self.mailboxes
                        .entry(recipient.clone())
                        .or_default()
                        .push(msg.clone());
                    ctx.console_logs.push(format!(
                        "[mail-server] SMTP stored mail from {} -> {}",
                        msg.from, recipient
                    ));
                    let _ = sock.send_slice(b"250 OK: Message queued\r\n");
                } else {
                    let _ = sock.send_slice(b"500 Syntax error\r\n");
                }
                sock.close();
            } else if sock.state() == tcp::State::CloseWait {
                sock.close();
            }
        }

        if let Some(h) = self.pop3_handle {
            let sock = ctx.sockets.get_mut::<tcp::Socket>(h);
            if !sock.is_open() {
                let _ = sock.listen(110);
            }
            if sock.can_recv() {
                let mut raw = Vec::new();
                let _ = sock.recv(|buf| {
                    raw.extend_from_slice(buf);
                    (buf.len(), ())
                });
                let user = String::from_utf8_lossy(&raw).trim().to_string();
                let msgs = self.mailboxes.get(&user).cloned().unwrap_or_default();
                let json_resp = serde_json::to_string(&msgs).unwrap_or_else(|_| "[]".into());
                let _ = sock.send_slice(json_resp.as_bytes());
                sock.close();
                ctx.console_logs
                    .push(format!("[mail-server] POP3 fetched {} msgs for {user}", msgs.len()));
            } else if sock.state() == tcp::State::CloseWait {
                sock.close();
            }
        }
    }

    fn handle_command(&mut self, cmd: Value, _ctx: &mut PluginContext) -> Result<Value, String> {
        let action = cmd.get("command").and_then(|v| v.as_str()).unwrap_or("");
        if action == "configure" {
            let cfg = cmd.get("config").unwrap_or(&cmd);
            if let Some(d) = cfg.get("domain").and_then(|v| v.as_str()) {
                self.domain = d.to_string();
            }
            return Ok(json!({ "ok": true }));
        }
        Err(format!("Unknown mail-server command: {action}"))
    }

    fn get_state(&self) -> Value {
        json!({
            "domain": self.domain,
            "mailboxes": self.mailboxes,
        })
    }
}

pub struct MailClientPlugin {
    handle: Option<SocketHandle>,
    local_port: u16,
    pending_mode: Option<String>, // "smtp" | "pop3"
    pending_payload: String,
    sent: bool,
    rx_buf: Vec<u8>,
    inbox: Vec<EmailMessage>,
    last_status: Option<String>,
}

impl MailClientPlugin {
    pub fn new() -> Self {
        Self {
            handle: None,
            local_port: 52000,
            pending_mode: None,
            pending_payload: String::new(),
            sent: false,
            rx_buf: Vec::new(),
            inbox: Vec::new(),
            last_status: None,
        }
    }

    pub fn metadata() -> PluginMetadata {
        PluginMetadata {
            id: "mail-client".into(),
            name: "Mail Client".into(),
            description: "Sends emails (SMTP/25) and checks inbox (POP3/110).".into(),
            config_schema: vec![],
            actions: vec![
                PluginActionSchema {
                    command: "send".into(),
                    label: "Send Email".into(),
                    fields: vec![
                        ConfigFieldSchema {
                            key: "server".into(),
                            label: "Mail Server (IP or hostname)".into(),
                            field_type: "text".into(),
                            default_value: json!("mail.corp"),
                        },
                        ConfigFieldSchema {
                            key: "from".into(),
                            label: "From".into(),
                            field_type: "text".into(),
                            default_value: json!("alice@corp"),
                        },
                        ConfigFieldSchema {
                            key: "to".into(),
                            label: "To".into(),
                            field_type: "text".into(),
                            default_value: json!("bob@corp"),
                        },
                        ConfigFieldSchema {
                            key: "subject".into(),
                            label: "Subject".into(),
                            field_type: "text".into(),
                            default_value: json!("Hello from NetLab"),
                        },
                        ConfigFieldSchema {
                            key: "body".into(),
                            label: "Body".into(),
                            field_type: "textarea".into(),
                            default_value: json!("Testing end-to-end SMTP over smoltcp!"),
                        },
                    ],
                },
                PluginActionSchema {
                    command: "fetch".into(),
                    label: "Fetch Inbox".into(),
                    fields: vec![
                        ConfigFieldSchema {
                            key: "server".into(),
                            label: "Mail Server (IP or hostname)".into(),
                            field_type: "text".into(),
                            default_value: json!("mail.corp"),
                        },
                        ConfigFieldSchema {
                            key: "user".into(),
                            label: "Mailbox Address".into(),
                            field_type: "text".into(),
                            default_value: json!("bob@corp"),
                        },
                    ],
                },
            ],
        }
    }
}

impl AppPlugin for MailClientPlugin {
    fn id(&self) -> &'static str {
        "mail-client"
    }

    fn name(&self) -> &'static str {
        "Mail Client"
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

        if let Some(mode) = &self.pending_mode {
            if sock.can_send() && !self.sent {
                let _ = sock.send_slice(self.pending_payload.as_bytes());
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
                    let resp = String::from_utf8_lossy(&self.rx_buf).trim().to_string();
                    if mode == "pop3" {
                        if let Ok(msgs) = serde_json::from_str::<Vec<EmailMessage>>(&resp) {
                            self.inbox = msgs;
                            self.last_status = Some(format!("Fetched {} messages", self.inbox.len()));
                            ctx.console_logs.push(format!(
                                "[mail-client] Inbox: {} messages",
                                self.inbox.len()
                            ));
                        }
                    } else {
                        self.last_status = Some(resp.clone());
                        ctx.console_logs.push(format!("[mail-client] SMTP: {resp}"));
                    }
                }
                sock.close();
                self.pending_mode = None;
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

        let target_ip: Ipv4Addr = if let Ok(ip) = server.parse() {
            ip
        } else if let Some(&cached) = ctx.dns_cache.get(&server.to_lowercase()) {
            cached
        } else {
            return Err(format!("Host '{server}' not in DNS cache. Run DNS lookup first."));
        };

        let (mode, port, payload) = match action {
            "send" => {
                let msg = EmailMessage {
                    from: cmd.get("from").and_then(|v| v.as_str()).unwrap_or("user@corp").into(),
                    to: cmd.get("to").and_then(|v| v.as_str()).unwrap_or("bob@corp").into(),
                    subject: cmd.get("subject").and_then(|v| v.as_str()).unwrap_or("(no subject)").into(),
                    body: cmd.get("body").and_then(|v| v.as_str()).unwrap_or("").into(),
                    timestamp_ms: ctx.now_ms,
                };
                ("smtp".to_string(), 25u16, serde_json::to_string(&msg).unwrap())
            }
            "fetch" => {
                let user = cmd.get("user").and_then(|v| v.as_str()).unwrap_or("bob@corp");
                ("pop3".to_string(), 110u16, format!("{user}\n"))
            }
            _ => return Err(format!("Unknown mail-client command: {action}")),
        };

        let h = self.handle.ok_or("Mail client socket not mounted")?;
        let sock = ctx.sockets.get_mut::<tcp::Socket>(h);
        sock.abort();
        self.local_port = self.local_port.wrapping_add(1).max(52000);
        let remote = IpEndpoint::new(
            IpAddress::Ipv4(Ipv4Address::from_octets(target_ip.octets())),
            port,
        );
        sock.connect(ctx.iface.context(), remote, self.local_port)
            .map_err(|e| format!("TCP connect failed: {e:?}"))?;

        self.pending_mode = Some(mode);
        self.pending_payload = payload;
        self.sent = false;
        self.rx_buf.clear();
        Ok(json!({ "status": "connecting" }))
    }

    fn get_state(&self) -> Value {
        json!({
            "last_status": self.last_status,
            "inbox": self.inbox,
        })
    }
}
