//! `http::error` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::error",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the error message if the HTTP transaction failed.",
            &["http::error token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
