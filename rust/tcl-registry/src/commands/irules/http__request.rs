//! `HTTP::request` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::request",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the raw HTTP request headers as a string.",
            &["HTTP::request"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
