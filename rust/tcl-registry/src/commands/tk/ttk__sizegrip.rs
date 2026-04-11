//! `ttk::sizegrip` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::sizegrip",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate a themed size grip widget for resizing.",
            &["ttk::sizegrip pathName ?options?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
