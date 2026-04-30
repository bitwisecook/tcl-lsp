//! `CACHE::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the HTTP payload of the cache response.",
            &["CACHE::payload"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
