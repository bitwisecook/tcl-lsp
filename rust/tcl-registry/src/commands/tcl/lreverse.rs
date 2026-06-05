//! `lreverse` — reverse a list.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lreverse list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lreverse",
        const_fold: Some(crate::const_fold::fold_lreverse),
        traits: Traits::FRAMELESS_RUNTIME | Traits::PURE,
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::exact(1),
        return_type: Some(TclType::List),
hover: Some(HoverSnippet {
    summary: "Reverse the order of a list",
    synopsis: &["lreverse list"],
    snippet: "The lreverse command returns a list that has the same elements as its input list, list, except with the elements in the reverse order.",
    source: "Tcl man page lreverse.n",
    examples: "",
    return_value: "",
}),
        forms: FORMS,
        arg_types: &[(0, ArgTypeHint { expected: Some(TclType::List), shimmers: true })],
        ..CommandSpec::DEFAULT
    }
}
