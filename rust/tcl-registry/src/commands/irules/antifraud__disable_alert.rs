//! `ANTIFRAUD::disable_alert` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::disable_alert",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disables the current alert.",
            &["ANTIFRAUD::disable_alert"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
