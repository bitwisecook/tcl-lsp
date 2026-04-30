//! `safe::interpInit` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "safe::interpInit",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Initialise an existing interpreter as a safe interpreter.",
            &["safe::interpInit child ?options...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
