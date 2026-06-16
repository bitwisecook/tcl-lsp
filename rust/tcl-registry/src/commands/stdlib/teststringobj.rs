//! `teststringobj` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "teststringobj",
        dialects: None,
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Test string Tcl_Obj operations.",
            synopsis: &["teststringobj"],
            snippet: "",
            source: "Tcl test binary (tclTest.c)",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
