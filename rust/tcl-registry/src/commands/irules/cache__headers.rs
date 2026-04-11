//! `CACHE::headers` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::headers",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the HTTP headers of the object in the cache.",
            &["CACHE::headers"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
