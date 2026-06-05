//! `bgerror` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "bgerror",
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief("Handle background errors", &[], "Tcl")),
        ..CommandSpec::DEFAULT
    }
}
