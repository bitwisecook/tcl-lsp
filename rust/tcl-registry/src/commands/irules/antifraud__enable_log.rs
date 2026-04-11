//! `ANTIFRAUD::enable_log` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::enable_log",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enables Anti-Fraud TMM logs for the current transaction.",
            &["ANTIFRAUD::enable_log (LOG_LEVEL)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
