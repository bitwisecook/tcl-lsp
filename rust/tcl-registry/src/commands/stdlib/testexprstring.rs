//! `testexprstring` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testexprstring",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_ExprString.",
            &["testexprstring"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
