//! `safe::setSyncMode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "safe::setSyncMode",
        dialects: None,
        arity: Arity::new(0, 2),
        hover: Some(HoverSnippet {
            summary: "Set or query the synchronous-source mode for a safe interpreter.",
            synopsis: &["safe::setSyncMode ?child? ?boolean?"],
            snippet: "",
            source: "Tcl stdlib Safe Base",
            examples: "",
            return_value: "",
        }),
        required_package: Some("safe"),
        ..CommandSpec::DEFAULT
    }
}
