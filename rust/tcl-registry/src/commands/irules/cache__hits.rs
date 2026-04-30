//! `CACHE::hits` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::hits",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the document cache hits.",
            &["CACHE::hits"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
