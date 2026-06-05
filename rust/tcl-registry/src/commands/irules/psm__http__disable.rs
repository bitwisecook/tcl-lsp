//! `PSM::HTTP::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSM::HTTP::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "To disable PSM for HTTP traffic.",
            synopsis: &["PSM::HTTP::disable"],
            snippet: "To disable PSM for HTTP traffic",
            source: "https://clouddocs.f5.com/api/irules/PSM__HTTP__disable.html",
            examples: "when HTTP_REQUEST {\n    if { [HTTP::uri] starts_with \"/bypass\" } {\n        PSM::HTTP::disable\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &["CLIENT_ACCEPTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
