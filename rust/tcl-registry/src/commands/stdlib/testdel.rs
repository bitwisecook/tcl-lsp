//! `testdel` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testdel",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test command deletion callbacks.",
            &["testdel"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
