//! `ANTIFRAUD::alert_forbidden_added_element` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_forbidden_added_element",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Deprecated: For the external_sources alert: returns forbidden added HTML element and its content, in an escaped base64 format.",
            synopsis: &["ANTIFRAUD::alert_forbidden_added_element"],
            snippet: "For the external_sources alert: returns forbidden added HTML element and its content, in an escaped base64 format.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_forbidden_added_element.html",
            examples: "",
            return_value: "",
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
