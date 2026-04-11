//! `tcl::OptKeyDelete` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::OptKeyDelete",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Delete a previously registered option description.",
            &["tcl::OptKeyDelete key"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
