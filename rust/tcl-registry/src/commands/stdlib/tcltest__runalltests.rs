//! `tcltest::runAllTests` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::runAllTests",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Source all test files matching the configured patterns.",
            &["tcltest::runAllTests"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
