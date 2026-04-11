//! `http::responseHeaderValue` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::responseHeaderValue",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Return the value of a specific HTTP response header.",
            &["http::responseHeaderValue token name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
