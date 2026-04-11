//! `HTTP::uri` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::uri",
        traits: Traits::PURE | Traits::CSE_CANDIDATE | Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Returns or sets the URI part of the HTTP request.",
            &["HTTP::uri (URI)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
