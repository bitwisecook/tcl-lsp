//! `ttk::combobox` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::combobox",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate a themed combobox widget.",
            &["ttk::combobox pathName ?options?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
