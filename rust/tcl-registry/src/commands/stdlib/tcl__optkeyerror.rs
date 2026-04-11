//! `tcl::OptKeyError` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::OptKeyError",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Generate an error message for a registered option description.",
            &["tcl::OptKeyError key ?prefix?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
