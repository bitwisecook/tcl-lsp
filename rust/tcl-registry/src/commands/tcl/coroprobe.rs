//! `coroprobe` — probe a suspended coroutine.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "coroprobe",
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(2),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Evaluate a command in a suspended coroutine.",
            &["coroprobe coroName command ?arg ...?"],
            "Tcl coroprobe(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
