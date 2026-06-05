//! `TclOO` class variable.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "classvariable variableName ?variableName ...?",
}];

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
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
