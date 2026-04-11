//! `oo::objdefine` — define per-object members.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::objdefine",
        traits: Traits::LANGUAGE_KEYWORD | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(2),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Define per-object members.",
            &["oo::objdefine objectName ?definition?"],
            "Tcl oo::objdefine(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
