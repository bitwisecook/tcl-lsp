//! `http::code` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::code",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the HTTP status line (e.g. ``HTTP/1.1 200 OK``).",
            &["http::code token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
