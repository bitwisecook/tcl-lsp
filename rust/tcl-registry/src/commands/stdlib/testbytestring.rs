//! `testbytestring` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testbytestring",
        dialects: None,
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Create a bytestring Tcl_Obj from a string.",
            synopsis: &["testbytestring"],
            snippet: "",
            source: "Tcl test binary (tclTest.c)",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
