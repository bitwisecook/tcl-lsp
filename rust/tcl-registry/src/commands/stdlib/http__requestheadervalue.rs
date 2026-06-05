//! `http::requestHeaderValue` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::requestHeaderValue",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Return the value of a specific HTTP request header.",
            synopsis: &["http::requestHeaderValue token name"],
            snippet: "",
            source: "Tcl stdlib http package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("http"),
        ..CommandSpec::DEFAULT
    }
}
