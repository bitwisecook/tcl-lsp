//! `auto_mkindex` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_mkindex",
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Generate tclIndex from Tcl source files",
            &[],
            "Tcl",
        )),
        ..CommandSpec::DEFAULT
    }
}
