//! `HTTP2::header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet::brief(
            "Queries or modifies HTTP/2 pseudo-headers.",
            &["HTTP2::header <name>"],
            "F5 iRules",
        )),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["HTTP"],
            also_in: &["MR_EGRESS", "MR_INGRESS", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
