//! `coroinject` — inject a command into a coroutine.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "coroinject",
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(2),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Inject a command into a suspended coroutine.",
            &["coroinject coroName command ?arg ...?"],
            "Tcl coroinject(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
