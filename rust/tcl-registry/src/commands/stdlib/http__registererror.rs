//! `http::registerError` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::registerError",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Register or retrieve an error message for a protocol handler.",
            &["http::registerError token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
