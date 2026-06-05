//! `pkg::create` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pkg::create",
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create a package ifneeded script",
            &[],
            "Tcl",
        )),
        ..CommandSpec::DEFAULT
    }
}
