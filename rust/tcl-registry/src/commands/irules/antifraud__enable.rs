//! `ANTIFRAUD::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enables the anti-fraud plugin.",
            &["ANTIFRAUD::enable (ANTIFRAUD_PROFILE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
