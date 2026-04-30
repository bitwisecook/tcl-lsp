//! `HTTP::username` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::username",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the username part of HTTP basic authentication.",
            &["HTTP::username"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
