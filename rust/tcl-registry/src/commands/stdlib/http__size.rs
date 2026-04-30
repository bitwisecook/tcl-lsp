//! `http::size` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::size",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the number of bytes received so far.",
            &["http::size token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
