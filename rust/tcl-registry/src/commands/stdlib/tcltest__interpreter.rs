//! `tcltest::interpreter` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::interpreter",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set the path of the interpreter for subprocess tests.",
            &["tcltest::interpreter ?interp?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
