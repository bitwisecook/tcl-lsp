//! `DIAMETER::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or sets DIAMETER message payload.",
            synopsis: &["DIAMETER::payload ('replace' PAYLOAD)?"],
            snippet: "This iRule command gets or sets the current DIAMETER message\\'s\npayload, as a byte string.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__payload.html",
            examples: "when DIAMETER_INGRESS {\n    log local0. \"Received a DIAMETER message, with payload [DIAMETER::payload]\"\n}",
            return_value: "",
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
