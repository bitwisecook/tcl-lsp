//! `listbox` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "listbox",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate a listbox widget.",
            &["listbox pathName ?option value ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
