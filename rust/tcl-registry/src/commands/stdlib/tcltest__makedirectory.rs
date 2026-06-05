//! `tcltest::makeDirectory` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::makeDirectory",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Create a temporary test directory.",
            synopsis: &["tcltest::makeDirectory name ?directory?"],
            snippet: "",
            source: "Tcl stdlib tcltest package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        ..CommandSpec::DEFAULT
    }
}
