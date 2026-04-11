//! `tcltest::singleProcess` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::singleProcess",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set single-process mode.  Deprecated: use ``configure -singleproc``.",
            &["tcltest::singleProcess ?boolean?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
