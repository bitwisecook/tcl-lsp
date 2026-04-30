//! `TclOO` class.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::configurable",
        traits: Traits::IS_OO_METACLASS | Traits::LANGUAGE_KEYWORD | Traits::DEFINES_PROCEDURE,
        dialects: Some(DialectSet::TCL90),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Define a configurable TclOO class.",
            &["oo::configurable create name ?definition?"],
            "Tcl oo::configurable(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
