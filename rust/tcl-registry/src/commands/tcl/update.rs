//! `update` — process pending events.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "update",
        arity: Arity::new(0, 1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Process pending events and idle callbacks.",
            &["update ?idletasks?"],
            "Tcl update(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
