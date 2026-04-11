//! `CACHE::expire` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::expire",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Forces the document to be revalidated from the server.",
            &["CACHE::expire"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
