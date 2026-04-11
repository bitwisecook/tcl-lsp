//! `yieldto` — yield to a command from a coroutine.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yieldto",
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Yield to a command from a coroutine.",
            &["yieldto command ?arg ...?"],
            "Tcl yieldto(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
