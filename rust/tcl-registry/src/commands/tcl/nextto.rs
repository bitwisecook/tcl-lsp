//! `nextto` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "nextto",
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "invoke a specific superclass implementation of a method",
            &["nextto class ?arg ...?"],
            "Tcl man page next.n",
        )),
        ..CommandSpec::DEFAULT
    }
}
