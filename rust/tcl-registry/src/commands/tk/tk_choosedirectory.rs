//! `tk_chooseDirectory` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tk_chooseDirectory",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Pop up a dialogue for the user to select a directory.",
            &["tk_chooseDirectory ?option value ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
