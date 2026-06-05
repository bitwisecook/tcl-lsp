//! `DIAMETER::is_response` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::is_response",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns true if it is a DIAMETER response, otherwise, returns false.",
            synopsis: &["DIAMETER::is_response"],
            snippet: "This iRule command returns true if the current message is a DIAMETER response.\nOtherwise, it returns false.\n\nThis command is the exact inverse of DIAMETER::is_request.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__is_response.html",
            examples: "when DIAMETER_INGRESS {\n    if { [DIAMETER::is_response] } {\n        log local0. \"Response received\"\n    }\n}",
            return_value: "TRUE or FALSE",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER", "MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
