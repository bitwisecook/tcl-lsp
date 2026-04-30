//! `teststaticlibrary` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "teststaticlibrary",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_StaticLibrary (9.0+).",
            &["teststaticlibrary"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
