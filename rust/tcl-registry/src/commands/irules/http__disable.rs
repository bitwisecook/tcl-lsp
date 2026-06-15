//! `HTTP::disable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Changes the HTTP filter from full parsing to passthrough mode.",
            synopsis: &["HTTP::disable (discard)?"],
            snippet: "Changes the HTTP filter from full parsing to passthrough mode. This\ncommand is useful when using an HTTP profile with an application that\nproxies data over HTTP. One use of this command is when you need to\ntunnel PPP over HTTP and disable HTTP processing once the connection\nhas been established.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__disable.html",
            examples: "when HTTP_REQUEST {\npersist hash $key\nif { [string toupper [HTTP::method]] eq \"CONNECT\" } {\n\n      # Proxy connect method should continue as a passthrough\n      HTTP::disable\n\n      # Ask the server to close the connection after this request\n      HTTP::header replace Connection close\n   }\n}",
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
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "HTTP::disable (discard)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpHeader,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
