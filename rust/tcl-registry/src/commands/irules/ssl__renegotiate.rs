//! `SSL::renegotiate` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::renegotiate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Controls renegotiation of an SSL connection.",
            synopsis: &["SSL::renegotiate (enable | disable)?"],
            snippet: "Controls renegotiation of an SSL connection, often used to enforce new encryption settings or certificate requirements.\n\nThis command has different results depending on whether the BIG-IP system evaluates the command under a client-side or a server-side context. The command only succeeds if SSL is enabled on the connection; otherwise, the command returns an error.",
            source: "https://clouddocs.f5.com/api/irules/SSL__renegotiate.html",
            examples: "when CLIENTSSL_HANDSHAKE {\n    SSL::renegotiate disable\n}",
            return_value: "SSL::renegotiate Renegotiates a client-side or server-side SSL connection, depending on the context. When the system evaluates the command under a client-side context, the system immediately renegotiates a request for the associated client-side connection, if client-side renegotiation is enabled.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["CLIENTSSL", "SERVERSSL"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
