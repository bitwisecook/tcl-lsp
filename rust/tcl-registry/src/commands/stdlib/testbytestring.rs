//! `testbytestring` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testbytestring",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Create a bytestring Tcl_Obj from a string.",
            &["testbytestring"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
