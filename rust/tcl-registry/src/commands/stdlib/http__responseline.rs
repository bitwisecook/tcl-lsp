//! `http::responseLine` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::responseLine",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the HTTP response status line.",
            &["http::responseLine token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
