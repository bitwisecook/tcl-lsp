//! `tcltest::removeFile` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::removeFile",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Remove a temporary test file.",
            &["tcltest::removeFile name ?directory?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
