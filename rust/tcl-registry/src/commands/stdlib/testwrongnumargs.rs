//! `testwrongnumargs` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testwrongnumargs",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Test Tcl_WrongNumArgs.",
            synopsis: &["testwrongnumargs"],
            snippet: "",
            source: "Tcl test binary (tclTest.c)",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
