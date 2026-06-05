//! `tcltest::customMatch` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::customMatch",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Register a custom matching command for test results.",
            synopsis: &["tcltest::customMatch mode command"],
            snippet: "",
            source: "Tcl stdlib tcltest package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        ..CommandSpec::DEFAULT
    }
}
