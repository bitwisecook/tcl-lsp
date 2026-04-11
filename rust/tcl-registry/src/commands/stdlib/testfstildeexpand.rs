//! `testfstildeexpand` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testfstildeexpand",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test tilde expansion in file paths (9.0+).",
            &["testfstildeexpand"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
