//! `testevent` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testevent",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_QueueEvent and event processing.",
            &["testevent"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
