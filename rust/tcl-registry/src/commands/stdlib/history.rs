//! `history` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "history",
        traits: Traits::UNSAFE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Manipulate the history list of previously executed commands.",
            &["history"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
