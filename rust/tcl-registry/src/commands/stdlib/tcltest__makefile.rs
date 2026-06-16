//! `tcltest::makeFile` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::makeFile",
        dialects: None,
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet {
            summary: "Create a temporary test file with the given contents.",
            synopsis: &["tcltest::makeFile contents name ?directory?"],
            snippet: "",
            source: "Tcl stdlib tcltest package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        ..CommandSpec::DEFAULT
    }
}
