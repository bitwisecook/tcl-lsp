//! `http_method` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http_method",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the action of the HTTP request.",
            &["http_method"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
