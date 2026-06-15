//! `http::formatQuery` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::formatQuery",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet {
            summary: "Generate an x-url-encoded query string from key/value pairs.",
            synopsis: &["http::formatQuery key value ?key value ...?"],
            snippet: "Takes alternating key/value arguments and returns a properly URL-encoded query string suitable for use as a ``-query`` value in ``http::geturl``.",
            source: "Tcl stdlib http package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("http"),
        ..CommandSpec::DEFAULT
    }
}
