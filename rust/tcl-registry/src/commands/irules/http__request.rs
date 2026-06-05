//! `HTTP::request` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::request",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the raw HTTP request headers as a string.",
            synopsis: &["HTTP::request"],
            snippet: "Returns the raw HTTP request headers as a string.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__request.html",
            examples: "when HTTP_RESPONSE {\n    if {$lookup == 1 }{\n        # collect first response (from lookup server) only\n        HTTP::collect 1\n    }\n}",
            return_value: "Returns the raw HTTP request headers as a string. You can access the request payload (e.g. POST data) by triggering payload collection with the [HTTP::collect] command and then using [HTTP::payload] in the [HTTP_REQUEST_DATA] event.",
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
