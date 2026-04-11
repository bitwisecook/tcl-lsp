//! `tcltest::removeDirectory` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::removeDirectory",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Remove a temporary test directory.",
            &["tcltest::removeDirectory name ?directory?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
