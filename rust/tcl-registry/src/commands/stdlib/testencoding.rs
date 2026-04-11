//! `testencoding` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testencoding",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test encoding operations.",
            &["testencoding"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
