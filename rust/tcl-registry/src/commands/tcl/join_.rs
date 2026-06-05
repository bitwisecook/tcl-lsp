//! `join` — join list elements into a string.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "join list ?joinString?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "join",
        const_fold: Some(crate::const_fold::fold_join),
        traits: Traits::FRAMELESS_RUNTIME | Traits::PURE | Traits::CSE_CANDIDATE,
        arity: Arity::new(1, 2),
        return_type: Some(TclType::String),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
            },
        )],
        hover: Some(HoverSnippet {
            summary: "Create a string by joining together list elements",
            synopsis: &["join list ?joinString?"],
            snippet: "The list argument must be a valid Tcl list.",
            source: "Tcl man page join.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
