//! `pwd` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pwd",
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Return current working directory",
            &[],
            "Tcl",
        )),
        ..CommandSpec::DEFAULT
    }
}
