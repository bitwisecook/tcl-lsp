//! `tcltest::workingDirectory` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::workingDirectory",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary: "Get or set the working directory for tests.",
            synopsis: &["tcltest::workingDirectory ?path?"],
            snippet: "",
            source: "Tcl stdlib tcltest package (deprecated)",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        ..CommandSpec::DEFAULT
    }
}
