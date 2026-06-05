//! `auto_execok` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_execok",
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return path of executable, or empty string",
            &[],
            "Tcl",
        )),
        ..CommandSpec::DEFAULT
    }
}
