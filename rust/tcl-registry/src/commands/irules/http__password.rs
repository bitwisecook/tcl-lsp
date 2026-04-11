//! `HTTP::password` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::password",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the password part of HTTP basic authentication.",
            &["HTTP::password"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
