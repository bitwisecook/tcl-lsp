//! `DIAMETER::message` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::message",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the whole Diameter message as a TCL string object.",
            synopsis: &["DIAMETER::message"],
            snippet: "This iRule command returns the current Diameter message as a TCL\nstring object.  This includes both the header and the payload.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__message.html",
            examples: "when DIAMETER_INGRESS {\n    log local0. \"Received a DIAMETER message: [DIAMETER::message]\"\n}",
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
