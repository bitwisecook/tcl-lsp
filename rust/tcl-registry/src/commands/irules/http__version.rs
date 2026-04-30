//! `HTTP::version` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::version",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns or sets the HTTP version of the request or response.",
            &["HTTP::version ('0.9' | '1.0' | '1.1')?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
