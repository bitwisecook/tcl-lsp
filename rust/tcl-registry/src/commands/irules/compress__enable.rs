//! `COMPRESS::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "COMPRESS::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enables compression for the current HTTP response.",
            &["COMPRESS::enable (request | response)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
