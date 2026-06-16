//! `tcltest::bytestring` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::bytestring",
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Convert a string to its byte representation (Tcl < 9.0).",
            synopsis: &["tcltest::bytestring string"],
            snippet: "Equivalent to ``encoding convertfrom identity``.  Not exported in Tcl 9.0+.",
            source: "Tcl stdlib tcltest package (deprecated)",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        deprecated_replacement: Some("encoding convertfrom identity"),
        ..CommandSpec::DEFAULT
    }
}
