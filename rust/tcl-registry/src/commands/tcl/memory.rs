//! `memory` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "memory",
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Memory debugging (debug builds only)",
            &[],
            "Tcl",
        )),
        ..CommandSpec::DEFAULT
    }
}
