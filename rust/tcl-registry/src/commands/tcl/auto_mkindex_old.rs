//! `auto_mkindex_old` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_mkindex_old",
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief("Legacy tclIndex generator", &[], "Tcl")),
        ..CommandSpec::DEFAULT
    }
}
