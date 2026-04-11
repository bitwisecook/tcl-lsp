//! `fasthash` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fasthash",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns a hash for the specified string.",
            &["fasthash DATA"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
