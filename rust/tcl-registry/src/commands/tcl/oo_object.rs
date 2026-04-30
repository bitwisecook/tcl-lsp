//! `TclOO` object.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::object",
        traits: Traits::IS_OO_METACLASS,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "The root TclOO object.",
            &["oo::object"],
            "Tcl oo::object(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
