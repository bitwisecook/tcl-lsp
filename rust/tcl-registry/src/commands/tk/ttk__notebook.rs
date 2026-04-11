//! `ttk::notebook` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::notebook",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate a themed tabbed notebook widget.",
            &["ttk::notebook pathName ?options?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
