//! `tcltest::normalizeMsg` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::normalizeMsg",
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Normalise an error message for comparison (lowercase, strip trailing newline).",
            synopsis: &["tcltest::normalizeMsg msg"],
            snippet: "",
            source: "Tcl stdlib tcltest package (deprecated)",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        deprecated_replacement: Some("tcltest::customMatch"),
        ..CommandSpec::DEFAULT
    }
}
