//! `http::requestHeaders` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::requestHeaders",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Return the HTTP request headers as a list.",
            synopsis: &[
                "http::requestHeaders token",
                "http::requestHeaders token ?name?",
            ],
            snippet: "",
            source: "Tcl stdlib http package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("http"),
        ..CommandSpec::DEFAULT
    }
}
