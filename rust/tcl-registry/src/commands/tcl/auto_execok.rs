//! `auto_execok` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_execok",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        traits: Traits::OVERRIDABLE_LIBRARY_PROC,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return path of executable, or empty string",
            synopsis: &[],
            snippet: "",
            source: "",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
