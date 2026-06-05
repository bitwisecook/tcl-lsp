//! `safe::interpConfigure` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "safe::interpConfigure",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Get or set the configuration of a safe interpreter.",
            synopsis: &["safe::interpConfigure child ?options...?"],
            snippet: "",
            source: "Tcl stdlib Safe Base",
            examples: "",
            return_value: "",
        }),
        required_package: Some("safe"),
        ..CommandSpec::DEFAULT
    }
}
