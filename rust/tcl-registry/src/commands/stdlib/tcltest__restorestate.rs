//! `tcltest::restoreState` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::restoreState",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet {
            summary: "Restore interpreter state saved by ``saveState``.",
            synopsis: &["tcltest::restoreState"],
            snippet: "",
            source: "Tcl stdlib tcltest package (v1 compat)",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        ..CommandSpec::DEFAULT
    }
}
