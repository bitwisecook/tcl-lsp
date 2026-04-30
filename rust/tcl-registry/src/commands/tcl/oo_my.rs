//! `my` — call a method on the current object.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "my",
        traits: Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Call a method on the current object.",
            &["my method ?arg ...?"],
            "Tcl my(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
