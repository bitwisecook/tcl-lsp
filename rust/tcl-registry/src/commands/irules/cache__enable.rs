//! `CACHE::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Forces the document to be cached.",
            &["CACHE::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
