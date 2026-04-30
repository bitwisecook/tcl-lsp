//! `testcmdtoken` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testcmdtoken",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test command token operations.",
            &["testcmdtoken"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
