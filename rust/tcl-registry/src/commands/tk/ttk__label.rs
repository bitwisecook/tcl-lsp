//! `ttk::label` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::label",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate a themed label widget.",
            &["ttk::label pathName ?options?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
