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
        hover: Some(HoverSnippet {
            summary: "link local variables to class-shared variables",
            synopsis: &[
                "classvariable variableName ?variableName ...?",
                "classvariable variableName ?...?",
            ],
            snippet: "The classvariable command arranges for local variables with the given names to refer to variables shared among all instances of the class. Used inside method bodies or class definitions.",
            source: "Tcl man page define.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
