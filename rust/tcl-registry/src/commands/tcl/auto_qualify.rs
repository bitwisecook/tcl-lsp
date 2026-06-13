//! `auto_qualify` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_qualify",
        traits: Traits::OVERRIDABLE_LIBRARY_PROC,
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Compute fully-qualified names for auto-loading",
            synopsis: &[],
            snippet: "",
            source: "",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
