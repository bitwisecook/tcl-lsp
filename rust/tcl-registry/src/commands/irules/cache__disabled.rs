//! `CACHE::disabled` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::disabled",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns state of cache disable flag",
            &["CACHE::disabled"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
