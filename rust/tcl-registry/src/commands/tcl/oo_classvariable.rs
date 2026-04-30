//! `TclOO` class variable.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "classvariable",
        traits: Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Create a link to a class variable.",
            &["classvariable variableName ?...?"],
            "Tcl classvariable(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
