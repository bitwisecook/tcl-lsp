//! `tcl::OptKeyParse` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::OptKeyParse",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Parse arguments using a previously registered option description.",
            &["tcl::OptKeyParse key arglist"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
