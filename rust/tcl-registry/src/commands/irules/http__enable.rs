//! `HTTP::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Changes the HTTP filter from passthrough to full parsing mode.",
            &["HTTP::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
