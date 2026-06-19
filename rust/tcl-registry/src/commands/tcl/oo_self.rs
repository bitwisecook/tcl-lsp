//! `TclOO` context.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "self ?subcommand?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "self",
        traits: Traits::PURE | Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::new(0, 1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "query the identity of the current object",
            synopsis: &["self ?subcommand?"],
            snippet: "The self command is used within the body of a method to query the identity or other properties of the current object.",
            source: "Tcl man page self.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
