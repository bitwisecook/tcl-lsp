//! `auto_mkindex_old` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_mkindex_old",
        traits: Traits::OVERRIDABLE_LIBRARY_PROC,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Legacy tclIndex generator",
            synopsis: &[],
            snippet: "",
            source: "",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
