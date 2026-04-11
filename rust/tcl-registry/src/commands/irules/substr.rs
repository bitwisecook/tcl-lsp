//! `substr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "substr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns a substring from a string.",
            &["substr STRING SKIP_COUNT (TERMINATOR)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
