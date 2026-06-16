//! `auto_mkindex` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_mkindex",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        traits: Traits::OVERRIDABLE_LIBRARY_PROC,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Generate tclIndex from Tcl source files",
            synopsis: &[],
            snippet: "",
            source: "",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
