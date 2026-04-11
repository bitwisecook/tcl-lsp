//! `testbigdata` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testbigdata",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Create large data objects for testing (9.0+).",
            &["testbigdata"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
