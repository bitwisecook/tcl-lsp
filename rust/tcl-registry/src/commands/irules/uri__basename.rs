//! `URI::basename` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::basename",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Extracts the basename part of a given uri string.",
            &["URI::basename URI_STRING"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
