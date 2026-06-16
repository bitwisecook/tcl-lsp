//! `tcl_findLibrary` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl_findLibrary",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        traits: Traits::OVERRIDABLE_LIBRARY_PROC,
        arity: Arity::new(5, 6),
        hover: Some(HoverSnippet {
            summary: "Locate a Tcl library directory",
            synopsis: &[],
            snippet: "",
            source: "",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
