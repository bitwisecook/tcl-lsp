//! `TclOO` object.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::copy",
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::new(1, 2),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Create a copy of a TclOO object.",
            &["oo::copy sourceObject ?targetObject?"],
            "Tcl oo::copy(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
