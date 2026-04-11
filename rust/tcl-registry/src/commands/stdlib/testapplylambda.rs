//! `testapplylambda` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testapplylambda",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test apply with lambda expressions.",
            &["testapplylambda"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
