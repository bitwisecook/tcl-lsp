//! `http::meta` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::meta",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return the HTTP response headers as a dict-like list.",
            synopsis: &["http::meta token"],
            snippet: "",
            source: "Tcl stdlib http package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("http"),
        deprecated_replacement: Some("http::responseHeaders"),
        ..CommandSpec::DEFAULT
    }
}
