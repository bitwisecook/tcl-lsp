//! `tcl::OptProc` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::OptProc",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(3),
        hover: Some(HoverSnippet::brief(
            "Define a proc with automatic option parsing.",
            &["tcl::OptProc name optlist body"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
