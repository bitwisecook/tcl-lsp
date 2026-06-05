//! `HTTP::proxy` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::proxy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Controls the application of HTTP proxy when using an Explicit HTTP profile.",
            synopsis: &["HTTP::proxy", "HTTP::proxy ('enable' | 'disable')", "HTTP::proxy 'uri-rewrite' ('enable' | 'disable')", "HTTP::proxy ('addr' | 'port' | 'rtdom' | 'exists' | 'iptuple')", "HTTP::proxy chain ?args?"],
            snippet: "When an Explicit HTTP profile is applied to a virtual server, HTTP::proxy allows control of whether the BIG-IP will handle the proxy of the connection locally or send it to a downstream pool for processing instead.\n\nThis functionality was introduced in v11.6, and is available for v11.5.1 via an Engineering Hotfix.\n\nHTTP::proxy allows inspection of the results of the DNS lookup used in the Explicit HTTP Proxy.\n\nWhen a HTTP Proxy Chaining profile is applied to a virtual server, HTTP::proxy chain may be used to control the CONNECT request used to connect to the next proxy in the chain.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__proxy.html",
            examples: "when HTTP_REQUEST {\n    log local0. \"[HTTP::method] [HTTP::uri]\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
