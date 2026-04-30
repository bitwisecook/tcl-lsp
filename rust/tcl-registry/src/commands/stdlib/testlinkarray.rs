//! `testlinkarray` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testlinkarray",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test linked array variables (9.0+).",
            &["testlinkarray"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
