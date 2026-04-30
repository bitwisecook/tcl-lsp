//! `ANTIFRAUD::disable_phishing` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::disable_phishing",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disables phishing detection for the current transaction.",
            &["ANTIFRAUD::disable_phishing"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
