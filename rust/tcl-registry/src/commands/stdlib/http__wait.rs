//! `http::wait` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::wait",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Wait for an HTTP transaction to complete.",
            &["http::wait token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
