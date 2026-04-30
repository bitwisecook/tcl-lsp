//! `testexprdouble` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testexprdouble",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_ExprDouble.",
            &["testexprdouble"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
