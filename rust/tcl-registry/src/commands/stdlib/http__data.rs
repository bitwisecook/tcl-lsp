//! `http::data` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::data",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the body of the HTTP response.",
            &["http::data token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
