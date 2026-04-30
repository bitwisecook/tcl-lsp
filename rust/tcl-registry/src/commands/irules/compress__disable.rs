//! `COMPRESS::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "COMPRESS::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disables compression for the current HTTP response.",
            &["COMPRESS::disable (request | response)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
