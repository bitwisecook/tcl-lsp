//! `list` — create a Tcl list.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "list ?arg arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "list",
        const_fold: Some(crate::const_fold::fold_list),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::PURE
            | Traits::PRODUCES_CANONICAL_LIST,
        arity: Arity::any(),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet::brief(
            "Create a Tcl list.",
            &["list ?arg ...?"],
            "Tcl list(1)",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
