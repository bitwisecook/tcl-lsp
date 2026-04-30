//! `http::register` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::register",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(3, 6),
        hover: Some(HoverSnippet::brief(
            "Register a protocol handler (e.g. https) with the http package.",
            &["http::register proto defaultport command"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
