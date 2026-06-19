//! `filename` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "filename",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        arity: Arity::any(),
        hover: Some(HoverSnippet {
            summary: "File name conventions",
            synopsis: &[],
            snippet: "",
            source: "",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
