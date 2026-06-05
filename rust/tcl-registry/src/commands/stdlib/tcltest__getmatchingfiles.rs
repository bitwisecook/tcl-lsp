//! `tcltest::getMatchingFiles` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::getMatchingFiles",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Return list of test files matching configured patterns.",
            synopsis: &["tcltest::getMatchingFiles ?directory ...?"],
            snippet: "",
            source: "Tcl stdlib tcltest package (v1 compat)",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        ..CommandSpec::DEFAULT
    }
}
