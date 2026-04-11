//! `testregexp` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testregexp",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test regular expression engine.",
            &["testregexp"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
