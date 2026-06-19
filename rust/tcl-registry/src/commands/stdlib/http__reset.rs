//! `http::reset` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::reset",
        dialects: None,
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Reset an HTTP transaction.",
            synopsis: &["http::reset token ?why?"],
            snippet: "Resets the HTTP transaction identified by *token*.  The optional *why* string (default ``reset``) is passed to the registered ``-command`` callback.",
            source: "Tcl stdlib http package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("http"),
        ..CommandSpec::DEFAULT
    }
}
