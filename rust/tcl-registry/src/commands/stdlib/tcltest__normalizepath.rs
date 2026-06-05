//! `tcltest::normalizePath` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::normalizePath",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Normalise a file path (8.4 compatibility shim for ``file normalize``).",
            synopsis: &["tcltest::normalizePath pathVar"],
            snippet: "",
            source: "Tcl stdlib tcltest package (deprecated)",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        arg_roles: &[(0, ArgRole::VarWrite)],
        ..CommandSpec::DEFAULT
    }
}
