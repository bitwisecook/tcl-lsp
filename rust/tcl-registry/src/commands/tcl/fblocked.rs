//! `fblocked` — test whether last input exhausted all data.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fblocked",
        traits: Traits::PURE,
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Test whether last input exhausted all data.",
            &["fblocked channelId"],
            "Tcl fblocked(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
