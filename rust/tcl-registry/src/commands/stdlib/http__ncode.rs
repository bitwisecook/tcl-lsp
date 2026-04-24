//! `http::ncode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::ncode",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the numeric HTTP status code (e.g. 200, 404).",
            &["http::ncode token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
