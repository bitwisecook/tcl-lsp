//! `safe::interpInit` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "safe::interpInit",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Initialise an existing interpreter as a safe interpreter.",
            synopsis: &["safe::interpInit child ?options...?"],
            snippet: "",
            source: "Tcl stdlib Safe Base",
            examples: "",
            return_value: "",
        }),
        required_package: Some("safe"),
        ..CommandSpec::DEFAULT
    }
}
