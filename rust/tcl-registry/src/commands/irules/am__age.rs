//! `AM::age` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AM::age",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `AM::age`.",
            &["AM::age"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
