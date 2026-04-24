//! `TclOO` class.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::class",
        traits: Traits::IS_OO_METACLASS | Traits::LANGUAGE_KEYWORD | Traits::DEFINES_PROCEDURE,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Define or manipulate a `TclOO` class.",
            &["oo::class create name ?definition?"],
            "Tcl oo::class(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
