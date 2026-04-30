//! `CACHE::statskey` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::statskey",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `CACHE::statskey`.",
            &["CACHE::statskey"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
