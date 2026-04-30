//! `tk_messageBox` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tk_messageBox",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Pop up a message window and wait for user response.",
            &["tk_messageBox ?option value ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
