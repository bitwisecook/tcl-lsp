//! `pkg::create` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pkg::create",
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create a package ifneeded script",
            synopsis: &[],
            snippet: "",
            source: "",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
