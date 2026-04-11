//! `CACHE::fresh` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::fresh",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns state of freshness flag for request.",
            &["CACHE::fresh"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
