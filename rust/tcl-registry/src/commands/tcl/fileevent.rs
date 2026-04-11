//! `fileevent` — set up channel event handlers.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileevent",

        arity: Arity::new(2, 3),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Set up channel event handlers.",
            &["fileevent channelId event ?script?"],
            "Tcl fileevent(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
