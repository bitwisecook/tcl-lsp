//! `testprint` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testprint",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test formatted output.",
            &["testprint"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
