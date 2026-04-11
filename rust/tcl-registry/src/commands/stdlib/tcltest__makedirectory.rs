//! `tcltest::makeDirectory` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::makeDirectory",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Create a temporary test directory.",
            &["tcltest::makeDirectory name ?directory?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
