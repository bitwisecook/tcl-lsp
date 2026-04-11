//! `tcltest::cleanupTests` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::cleanupTests",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Print statistics and clean up after a test file.",
            &["tcltest::cleanupTests"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
