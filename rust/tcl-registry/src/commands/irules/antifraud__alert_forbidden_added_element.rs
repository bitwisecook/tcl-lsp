//! `ANTIFRAUD::alert_forbidden_added_element` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_forbidden_added_element",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Deprecated: For the external_sources alert: returns forbidden added HTML element",
            &["ANTIFRAUD::alert_forbidden_added_element"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
