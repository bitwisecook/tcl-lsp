//! `testdoubledigits` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testdoubledigits",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test double-to-string digit conversion.",
            &["testdoubledigits"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
