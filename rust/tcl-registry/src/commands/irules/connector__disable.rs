//! `CONNECTOR::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CONNECTOR::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disable all the connectors on chain.",
            synopsis: &["CONNECTOR::disable"],
            snippet: "Disable all the connectors  on chain",
            source: "https://clouddocs.f5.com/api/irules/CONNECTOR__disable.html",
            examples: "when CLIENT_ACCEPTED {\n                CONNECTOR::disable\n            }",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
