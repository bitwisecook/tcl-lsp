//! `http::reasonPhrase` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::reasonPhrase",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the standard reason phrase for an HTTP status code.",
            &["http::reasonPhrase code"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
