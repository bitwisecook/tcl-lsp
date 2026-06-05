//! `http::ncode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::ncode",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return the numeric HTTP status code (e.g. 200, 404).",
            synopsis: &["http::ncode token"],
            snippet: "",
            source: "Tcl stdlib http package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("http"),
        ..CommandSpec::DEFAULT
    }
}
