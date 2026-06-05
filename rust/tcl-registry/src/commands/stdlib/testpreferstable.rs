//! `testpreferstable` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testpreferstable",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Test TIP 268 stable-vs-latest preference (9.0+).",
            synopsis: &["testpreferstable"],
            snippet: "",
            source: "Tcl test binary (tclTest.c)",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
