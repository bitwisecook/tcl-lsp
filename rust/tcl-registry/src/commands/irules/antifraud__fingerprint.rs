//! `ANTIFRAUD::fingerprint` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::fingerprint",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns fingerprint data, only in context of ANTIFRAUD_LOGIN event.",
            synopsis: &["ANTIFRAUD::fingerprint"],
            snippet: "Returns fingerprint data collected on client side.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__fingerprint.html",
            examples: "when ANTIFRAUD_LOGIN {\n                log local0. \"Client fingerprint: [ANTIFRAUD::fingerprint].\"\n            }",
            return_value: "Returns fingerprint data collected on client side.",
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
