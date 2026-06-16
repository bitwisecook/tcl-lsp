//! `http::code` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::code",
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return the HTTP status line (e.g. ``HTTP/1.1 200 OK``).",
            synopsis: &["http::code token"],
            snippet: "",
            source: "Tcl stdlib http package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("http"),
        deprecated_replacement: Some("http::responseLine"),
        ..CommandSpec::DEFAULT
    }
}
