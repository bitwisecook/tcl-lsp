//! `tcltest::interpreter` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::interpreter",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary: "Get or set the path of the interpreter for subprocess tests.",
            synopsis: &["tcltest::interpreter ?interp?"],
            snippet: "",
            source: "Tcl stdlib tcltest package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        ..CommandSpec::DEFAULT
    }
}
