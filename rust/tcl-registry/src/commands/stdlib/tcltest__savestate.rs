//! `tcltest::saveState` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::saveState",
        dialects: None,
        arity: Arity::exact(0),
        hover: Some(HoverSnippet {
            summary: "Save current interpreter state (procs and vars) for later restoration.",
            synopsis: &["tcltest::saveState"],
            snippet: "",
            source: "Tcl stdlib tcltest package (v1 compat)",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        ..CommandSpec::DEFAULT
    }
}
