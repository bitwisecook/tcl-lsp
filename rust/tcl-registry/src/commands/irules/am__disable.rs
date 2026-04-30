//! `AM::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AM::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `AM::disable`.",
            &["AM::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
