//! `safe::interpDelete` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "safe::interpDelete",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Delete a safe interpreter and release its resources.",
            &["safe::interpDelete child"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
