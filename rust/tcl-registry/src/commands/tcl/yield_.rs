//! `yield` — yield a value from a coroutine.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yield",
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::new(0, 1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Yield a value from a coroutine.",
            &["yield ?value?"],
            "Tcl yield(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
