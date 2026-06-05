//! `testfevent` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testfevent",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Test file event handling.",
            synopsis: &["testfevent"],
            snippet: "",
            source: "Tcl test binary (tclTest.c)",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
