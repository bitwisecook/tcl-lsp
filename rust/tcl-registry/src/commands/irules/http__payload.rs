//! `HTTP::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::payload",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Queries for or manipulates HTTP payload information.",
            &["HTTP::payload ( LENGTH | (OFFSET LENGTH) )?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
