//! `testappverifierpresent` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testappverifierpresent",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Check whether the app verifier is present (9.0+).",
            &["testappverifierpresent"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
