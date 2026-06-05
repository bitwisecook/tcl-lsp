//! `lreplace` — replace elements in a list.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lreplace list first last ?element element ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lreplace",
        traits: Traits::FRAMELESS_RUNTIME | Traits::BYTE_COMPILED | Traits::PURE,
        arity: Arity::at_least(3),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
hover: Some(HoverSnippet {
    summary: "Replace elements in a list with new elements",
    synopsis: &["lreplace list first last ?element element ...?", "lreplace list first last ?element ...?"],
    snippet: "lreplace returns a new list formed by replacing zero or more elements of list with the element arguments.",
    source: "Tcl man page lreplace.n",
    examples: "",
    return_value: "",
}),
        forms: FORMS,
        arg_types: &[(0, ArgTypeHint { expected: Some(TclType::List), shimmers: true }), (1, ArgTypeHint { expected: Some(TclType::Int), shimmers: true }), (2, ArgTypeHint { expected: Some(TclType::Int), shimmers: true })],
        ..CommandSpec::DEFAULT
    }
}
