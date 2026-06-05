//! `next` — call the next method in the chain.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "next ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "next",
        traits: Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::any(),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Call the next method in the chain.",
            &["next ?arg ...?"],
            "Tcl next(1)",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
