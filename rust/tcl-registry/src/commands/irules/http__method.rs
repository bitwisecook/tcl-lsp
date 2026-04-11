//! `HTTP::method` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::method",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the type of HTTP request method.",
            &["HTTP::method"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
