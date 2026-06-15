//! `tcltest::cleanupTests` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::cleanupTests",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet {
            summary: "Print statistics and clean up after a test file.",
            synopsis: &["tcltest::cleanupTests"],
            snippet: "Call at the end of each test file.  Prints a summary of passed/failed/skipped tests and performs clean-up.",
            source: "Tcl stdlib tcltest package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        ..CommandSpec::DEFAULT
    }
}
