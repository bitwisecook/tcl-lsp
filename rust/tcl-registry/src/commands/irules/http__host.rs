//! `HTTP::host` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::host",
        traits: Traits::PURE | Traits::CSE_CANDIDATE | Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Returns the value contained in the Host header of an HTTP request.",
            &["HTTP::host ?name?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
