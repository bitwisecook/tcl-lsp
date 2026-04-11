//! `ANTIFRAUD::disable_auto_transactions` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::disable_auto_transactions",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disables automatic transactions for the current transaction.",
            &["ANTIFRAUD::disable_auto_transactions"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
