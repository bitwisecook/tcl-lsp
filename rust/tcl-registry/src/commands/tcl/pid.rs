//! `pid` — return process ID(s).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pid",
        traits: Traits::PURE,
        arity: Arity::new(0, 1),
        return_type: Some(TclType::Int),
        hover: Some(HoverSnippet::brief(
            "Return process ID(s).",
            &["pid ?fileId?"],
            "Tcl pid(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
