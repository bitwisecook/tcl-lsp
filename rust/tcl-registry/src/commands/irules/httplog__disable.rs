//! `HTTPLOG::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTPLOG::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `HTTPLOG::disable`.",
            &["HTTPLOG::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
