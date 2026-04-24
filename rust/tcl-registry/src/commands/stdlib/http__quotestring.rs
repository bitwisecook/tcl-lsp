//! `http::quoteString` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::quoteString",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "URL-encode a single string.",
            &["http::quoteString string"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
