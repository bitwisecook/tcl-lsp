//! `tcl::tm::roots` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::tm::roots",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Set the root paths for Tcl module discovery.",
            &["tcl::tm::roots pathList"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
