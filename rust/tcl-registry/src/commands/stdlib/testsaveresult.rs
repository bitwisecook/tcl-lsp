//! `testsaveresult` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testsaveresult",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Test Tcl_SaveResult / Tcl_RestoreResult.",
            synopsis: &["testsaveresult"],
            snippet: "",
            source: "Tcl test binary (tclTest.c)",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
