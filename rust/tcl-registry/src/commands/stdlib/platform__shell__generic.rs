//! `platform::shell::generic` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "platform::shell::generic",
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return the generic platform identifier for a given Tcl shell.",
            synopsis: &["platform::shell::generic shell"],
            snippet: "",
            source: "Tcl stdlib platform package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("platform::shell"),
        ..CommandSpec::DEFAULT
    }
}
