//! `tcl::tm::path` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::tm::path",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Manage the list of paths searched for Tcl modules.",
            &["tcl::tm::path add ?path ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
