//! `testbumpinterpepoch` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testbumpinterpepoch",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Bump the interpreter compilation epoch.",
            &["testbumpinterpepoch"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
