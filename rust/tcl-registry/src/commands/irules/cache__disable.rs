//! `CACHE::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disables caching for this request.",
            &["CACHE::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
