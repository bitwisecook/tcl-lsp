//! `CACHE::age` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::age",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the age of the document in the cache.",
            &["CACHE::age"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
