//! `auto_reset` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_reset",
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief("Reset auto-loading state", &[], "Tcl")),
        ..CommandSpec::DEFAULT
    }
}
