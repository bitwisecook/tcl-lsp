//! `testpanic` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testpanic",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Test Tcl_Panic.", &["testpanic"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
