//! `safe::interpCreate` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "safe::interpCreate",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Create a safe child interpreter with restricted capabilities.",
            &["safe::interpCreate ?child? ?options...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
