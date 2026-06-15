//! `split` — split a string into a list.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "split string ?splitChars?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "split",
        const_fold: Some(crate::const_fold::fold_split),
        traits: Traits::FRAMELESS_RUNTIME | Traits::PURE | Traits::CSE_CANDIDATE,
        arity: Arity::new(1, 2),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Split a string into a proper Tcl list",
            synopsis: &["split string ?splitChars?"],
            snippet: "Returns a list created by splitting string at each character that is in the splitChars argument.",
            source: "Tcl man page split.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::String),
                shimmers: true,
            },
        )],
        ..CommandSpec::DEFAULT
    }
}
