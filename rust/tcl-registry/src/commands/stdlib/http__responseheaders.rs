//! `http::responseHeaders` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::responseHeaders",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Return the HTTP response headers as a list.",
            synopsis: &[
                "http::responseHeaders token",
                "http::responseHeaders token ?name?",
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
