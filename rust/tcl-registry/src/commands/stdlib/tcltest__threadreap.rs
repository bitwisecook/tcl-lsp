//! `tcltest::threadReap` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::threadReap",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet {
            summary: "Terminate all threads except the main thread and return the thread count.",
            synopsis: &["tcltest::threadReap"],
            snippet: "",
            source: "Tcl stdlib tcltest package (v1 compat)",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        ..CommandSpec::DEFAULT
    }
}
