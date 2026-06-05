//! `concat` — concatenate lists.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "concat ?arg arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "concat",
        const_fold: Some(crate::const_fold::fold_concat),
        traits: Traits::FRAMELESS_RUNTIME | Traits::PURE | Traits::PRODUCES_CANONICAL_LIST,
        arity: Arity::any(),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet::brief(
            "Join lists into a single list.",
            &["concat ?arg ...?"],
            "Tcl concat(1)",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
