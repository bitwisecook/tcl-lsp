//! `safe::interpConfigure` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "safe::interpConfigure",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Get or set the configuration of a safe interpreter.",
            &["safe::interpConfigure child ?options...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
