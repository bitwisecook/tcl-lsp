//! `ANTIFRAUD::result` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::result",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns result of login validation (passed or failed).",
            &["ANTIFRAUD::result"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
