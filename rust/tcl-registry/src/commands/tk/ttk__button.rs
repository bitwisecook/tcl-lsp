//! `ttk::button` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::button",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate a themed button widget.",
            &["ttk::button pathName ?options?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
