//! `memory` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "memory",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Memory debugging (debug builds only)",
            synopsis: &[],
            snippet: "",
            source: "",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
