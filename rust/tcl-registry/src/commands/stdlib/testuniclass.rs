//! `testuniclass` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testuniclass",
        dialects: None,
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Test Unicode character classification (9.0+).",
            synopsis: &["testuniclass"],
            snippet: "",
            source: "Tcl test binary (tclTest.c)",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
