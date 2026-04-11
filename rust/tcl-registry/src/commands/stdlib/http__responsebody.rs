//! `http::responseBody` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::responseBody",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the body of the HTTP response.",
            &["http::responseBody token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
