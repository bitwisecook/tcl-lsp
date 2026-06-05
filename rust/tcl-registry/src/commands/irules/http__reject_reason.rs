//! `HTTP::reject_reason` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::reject_reason",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the reason HTTP is aborting",
            synopsis: &["HTTP::reject_reason ('as_num')?"],
            snippet: "This returns the reason HTTP aborted the connection, either as a string, or as a numeric id suitable for an error code.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__reject_reason.html",
            examples: "when HTTP_REJECT {\n    log local0. \"HTTP Aborted:\" [HTTP::reject_reason]\n    log local0. \"Error code:\" [HTTP::reject_reason as_num]\n}",
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
