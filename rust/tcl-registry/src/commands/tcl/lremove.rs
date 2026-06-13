//! `lremove` — remove elements from a list.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lremove list ?index ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lremove",
        traits: Traits::PURE,
        dialects: Some(DialectSet::TCL90),
        arity: Arity::at_least(1),
        return_type: Some(TclType::List),
hover: Some(HoverSnippet {
    summary: "Remove elements from a list",
    synopsis: &["lremove list ?index ...?"],
    snippet: "The lremove command returns a new list formed by removing the elements at the given indices from list.",
    source: "Tcl man page lremove.n",
    examples: "",
    return_value: "",
}),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
