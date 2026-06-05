//! `SSL::nextproto` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::nextproto",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set the Next Protocol Negotiation (NPN) string.",
            synopsis: &["SSL::nextproto"],
            snippet: "Get or set the Next Protocol Negotiation (NPN) string.",
            source: "https://clouddocs.f5.com/api/irules/SSL__nextproto.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["CLIENTSSL"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
