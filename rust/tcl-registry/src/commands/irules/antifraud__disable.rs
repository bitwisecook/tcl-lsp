//! `ANTIFRAUD::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disables the anti-fraud plugin.",
            &["ANTIFRAUD::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
