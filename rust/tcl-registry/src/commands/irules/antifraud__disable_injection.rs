//! `ANTIFRAUD::disable_injection` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::disable_injection",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disables Anti-Fraud injections for the current transaction.",
            &["ANTIFRAUD::disable_injection"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
