//! `tcltest::test` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::test",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Define and run a single test case.",
            &["tcltest::test name description ?option value ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
