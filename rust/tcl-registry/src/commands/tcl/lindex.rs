//! `lindex` — retrieve an element from a list.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lindex list ?index ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lindex",
        const_fold: Some(crate::const_fold::fold_lindex),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::PURE
            | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Retrieve an element from a list",
            synopsis: &["lindex list ?index ...?"],
            snippet: "The lindex command accepts a parameter, list, which it treats as a Tcl list.",
            source: "Tcl man page lindex.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        arg_types: &[
            (
                0,
                ArgTypeHint {
                    expected: Some(TclType::List),
                    shimmers: true,
                },
            ),
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
        ],
        ..CommandSpec::DEFAULT
    }
}
