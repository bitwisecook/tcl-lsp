//! `testcmdtrace` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testcmdtrace",
        dialects: None,
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Test command tracing.",
            synopsis: &["testcmdtrace"],
            snippet: "",
            source: "Tcl test binary (tclTest.c)",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
