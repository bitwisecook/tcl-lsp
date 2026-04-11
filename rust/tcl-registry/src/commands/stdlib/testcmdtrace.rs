//! `testcmdtrace` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testcmdtrace",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test command tracing.",
            &["testcmdtrace"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
