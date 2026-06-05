//! `auto_qualify` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_qualify",
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Compute fully-qualified names for auto-loading",
            &[],
            "Tcl",
        )),
        ..CommandSpec::DEFAULT
    }
}
