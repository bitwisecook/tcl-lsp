//! `ANTIFRAUD::result` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::result",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns result of login validation (passed or failed).",
            synopsis: &["ANTIFRAUD::result"],
            snippet: "Returns result of login validation (passed or failed).",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__result.html",
            examples: "when ANTIFRAUD_LOGIN {\n                log local0. \"Username tried to log in with result [ANTIFRAUD::result].\"\n            }",
            return_value: "Returns result of login validation (passed or failed).",
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
