//! `http::status` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::status",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the status of an HTTP transaction (ok, reset, eof, error, timeout).",
            &["http::status token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
