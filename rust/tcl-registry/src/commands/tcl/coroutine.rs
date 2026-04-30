//! `coroutine` — create a coroutine.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "coroutine",
        traits: Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(2),
        arg_roles: &[(0, ArgRole::Name)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Create a coroutine.",
            &["coroutine name command ?arg ...?"],
            "Tcl coroutine(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
