//! `http::responseBody` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::responseBody",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return the body of the HTTP response.",
            synopsis: &["http::responseBody token"],
            snippet: "",
            source: "Tcl stdlib http package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("http"),
        ..CommandSpec::DEFAULT
    }
}
