//! `http::cleanup` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::cleanup",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Clean up the state associated with an HTTP connection.",
            &["http::cleanup token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
