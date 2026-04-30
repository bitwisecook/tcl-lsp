//! `tcltest::getMatchingFiles` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::getMatchingFiles",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Return list of test files matching configured patterns.",
            &["tcltest::getMatchingFiles ?directory ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
