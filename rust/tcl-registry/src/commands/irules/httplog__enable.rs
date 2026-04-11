//! `HTTPLOG::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTPLOG::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `HTTPLOG::enable`.",
            &["HTTPLOG::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
