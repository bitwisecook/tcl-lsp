//! `auto_import` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_import",
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Import auto-loaded commands into namespace",
            &[],
            "Tcl",
        )),
        ..CommandSpec::DEFAULT
    }
}
