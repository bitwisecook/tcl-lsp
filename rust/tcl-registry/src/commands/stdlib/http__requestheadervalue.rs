//! `http::requestHeaderValue` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::requestHeaderValue",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Return the value of a specific HTTP request header.",
            &["http::requestHeaderValue token name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
