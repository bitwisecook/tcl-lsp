//! `tcltest::makeFile` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::makeFile",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet::brief(
            "Create a temporary test file with the given contents.",
            &["tcltest::makeFile contents name ?directory?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
