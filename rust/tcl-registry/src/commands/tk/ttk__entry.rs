//! `ttk::entry` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::entry",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate a themed text entry widget.",
            &["ttk::entry pathName ?options?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
