//! `HTTP::redirect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::redirect",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Redirects an HTTP request or response to the specified URL.",
            &["HTTP::redirect REDIRECT_URL"],
            "F5 iRules",
        )),
        // GAP-D2: tainted redirect URL → open-redirect (IRULE3004).
        // Mirrors `irules/http__redirect.py`.
        taint_output_sink: Some("IRULE3004"),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &["LB_FAILED", "NAME_RESOLVED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
