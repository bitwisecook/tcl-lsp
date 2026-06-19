//! `auto_load` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_load",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        traits: Traits::OVERRIDABLE_LIBRARY_PROC,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Auto-load a command from the library",
            synopsis: &[],
            snippet: "",
            source: "",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
