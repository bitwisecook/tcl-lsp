//! `ANTIFRAUD::disable_alert` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::disable_alert",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disables the current alert.",
            synopsis: &["ANTIFRAUD::disable_alert"],
            snippet: "Disables the current alert",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__disable_alert.html",
            examples:
                "when ANTIFRAUD_ALERT {\n                ANTIFRAUD::disable_alert\n            }",
            return_value: "Disables the current alert",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ANTIFRAUD"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
