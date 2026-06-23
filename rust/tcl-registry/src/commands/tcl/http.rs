//! `http` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
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
