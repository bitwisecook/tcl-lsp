//! `fcopy` — copy data between channels.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fcopy",

        arity: Arity::at_least(2),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Copy data between channels.",
            &["fcopy inchan outchan ?-option value ...?"],
            "Tcl fcopy(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
