//! `ttk::separator` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::separator",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate a themed separator widget.",
            &["ttk::separator pathName ?options?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
