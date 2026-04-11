//! `http::meta` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::meta",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the HTTP response headers as a dict-like list.",
            &["http::meta token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
