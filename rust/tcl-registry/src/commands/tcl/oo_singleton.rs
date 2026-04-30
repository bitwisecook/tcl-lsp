//! `TclOO` class.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::singleton",
        traits: Traits::IS_OO_METACLASS | Traits::LANGUAGE_KEYWORD | Traits::DEFINES_PROCEDURE,
        dialects: Some(DialectSet::TCL90),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Define a singleton TclOO class.",
            &["oo::singleton create name ?definition?"],
            "Tcl oo::singleton(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
