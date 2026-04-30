//! `ttk::frame` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::frame",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate a themed frame container widget.",
            &["ttk::frame pathName ?options?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
