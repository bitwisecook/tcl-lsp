//! `accumulate` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "accumulate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::any(),
        hover: Some(HoverSnippet {
            summary: "Deprecated: use TCP::collect instead",
            synopsis: &[],
            snippet: "",
            source: "",
            examples: "",
            return_value: "",
        }),
        deprecated_replacement: Some("TCP::collect"),
        ..CommandSpec::DEFAULT
    }
}
