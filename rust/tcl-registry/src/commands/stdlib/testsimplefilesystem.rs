//! `testsimplefilesystem` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testsimplefilesystem",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test a simple virtual file system.",
            &["testsimplefilesystem"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
