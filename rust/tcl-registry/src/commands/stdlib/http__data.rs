//! `http::data` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::data",
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return the body of the HTTP response.",
            synopsis: &["http::data token"],
            snippet: "",
            source: "Tcl stdlib http package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("http"),
        deprecated_replacement: Some("http::responseBody"),
        ..CommandSpec::DEFAULT
    }
}
