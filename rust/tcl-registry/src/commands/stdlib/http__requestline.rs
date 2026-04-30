//! `http::requestLine` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::requestLine",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the HTTP request line sent.",
            &["http::requestLine token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
