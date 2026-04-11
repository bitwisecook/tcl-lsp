//! `tk_chooseColor` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tk_chooseColor",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Pop up a dialogue for the user to select a colour.",
            &["tk_chooseColor ?option value ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
