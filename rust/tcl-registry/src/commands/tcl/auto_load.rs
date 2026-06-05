//! `auto_load` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_load",
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Auto-load a command from the library",
            &[],
            "Tcl",
        )),
        ..CommandSpec::DEFAULT
    }
}
