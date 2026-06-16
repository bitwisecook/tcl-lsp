//! `http::cleanup` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::cleanup",
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Clean up the state associated with an HTTP connection.",
            synopsis: &["http::cleanup token"],
            snippet: "Releases all resources associated with *token*.  The token must not be used after this call.",
            source: "Tcl stdlib http package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("http"),
        ..CommandSpec::DEFAULT
    }
}
