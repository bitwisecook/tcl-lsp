//! `tcltest::viewFile` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::viewFile",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Return the contents of a file as a string.",
            &["tcltest::viewFile name ?directory?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
