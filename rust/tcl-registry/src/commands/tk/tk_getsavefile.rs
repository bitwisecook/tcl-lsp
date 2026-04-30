//! `tk_getSaveFile` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tk_getSaveFile",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Pop up a dialogue for the user to select a file to save.",
            &["tk_getSaveFile ?option value ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
