//! `platform::shell::identify` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "platform::shell::identify",
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return the platform identifier for a given Tcl shell.",
            synopsis: &["platform::shell::identify shell"],
            snippet: "",
            source: "Tcl stdlib platform package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("platform::shell"),
        ..CommandSpec::DEFAULT
    }
}
