//! `http` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http",
        arity: Arity::any(),
        hover: Some(HoverSnippet {
            summary: "HTTP client implementation (package http)",
            synopsis: &[],
            snippet: "",
            source: "",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
