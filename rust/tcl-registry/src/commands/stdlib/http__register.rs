//! `http::register` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::register",
        dialects: None,
        arity: Arity::new(3, 6),
        hover: Some(HoverSnippet {
            summary: "Register a protocol handler (e.g. https) with the http package.",
            synopsis: &["http::register proto defaultport command"],
            snippet: "Registers a handler for *proto* (e.g. ``https``).  When ``http::geturl`` encounters this protocol, it opens a socket via *command* on *defaultport*.",
            source: "Tcl stdlib http package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("http"),
        ..CommandSpec::DEFAULT
    }
}
