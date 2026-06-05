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
        hover: Some(HoverSnippet::brief(
            "Remove elements from a list.",
            &["lremove list ?index ...?"],
            "Tcl lremove(1)",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
