pub mod dhcp;
pub mod dns;
pub mod file;
pub mod http;
pub mod mail;

use crate::plugin::PluginRegistry;

pub fn register_builtin_plugins(registry: &mut PluginRegistry) {
    registry.register(dhcp::DhcpClientPlugin::metadata(), || {
        Box::new(dhcp::DhcpClientPlugin::new())
    });
    registry.register(dhcp::DhcpServerPlugin::metadata(), || {
        Box::new(dhcp::DhcpServerPlugin::new())
    });
    registry.register(dns::DnsServerPlugin::metadata(), || {
        Box::new(dns::DnsServerPlugin::new())
    });
    registry.register(dns::DnsResolverPlugin::metadata(), || {
        Box::new(dns::DnsResolverPlugin::new())
    });
    registry.register(http::HttpServerPlugin::metadata(), || {
        Box::new(http::HttpServerPlugin::new())
    });
    registry.register(http::HttpClientPlugin::metadata(), || {
        Box::new(http::HttpClientPlugin::new())
    });
    registry.register(file::FileServerPlugin::metadata(), || {
        Box::new(file::FileServerPlugin::new())
    });
    registry.register(file::FileClientPlugin::metadata(), || {
        Box::new(file::FileClientPlugin::new())
    });
    registry.register(mail::MailServerPlugin::metadata(), || {
        Box::new(mail::MailServerPlugin::new())
    });
    registry.register(mail::MailClientPlugin::metadata(), || {
        Box::new(mail::MailClientPlugin::new())
    });
}
