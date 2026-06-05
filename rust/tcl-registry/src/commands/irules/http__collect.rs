//! `HTTP::collect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::collect",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Collects an amount of HTTP body data that you specify.",
            &["HTTP::collect (CONTENT_LENGTH)?"],
            "F5 iRules",
        )),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[
                "AUTH_ERROR",
                "AUTH_FAILURE",
                "AUTH_RESULT",
                "AUTH_SUCCESS",
                "AUTH_WANTCREDENTIAL",
            ],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
