//! `AM::cache` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AM::cache",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `AM::cache`.",
            &["AM::cache"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
