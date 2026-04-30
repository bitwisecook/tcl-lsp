//! `HTTP::query` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::query",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 1),
        options: &[OptionSpec {
            name: "-normalized",
            takes_value: false,
            value_hint: "",
            detail: "Return the canonicalised query (URL evasion patterns rejected).",
        }],
        hover: Some(HoverSnippet::brief(
            "Returns or sets the query part of the HTTP request.",
            &["HTTP::query (QUERY_STRING)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
