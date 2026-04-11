//! `testsize` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testsize",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report sizeof various types.",
            &["testsize"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
