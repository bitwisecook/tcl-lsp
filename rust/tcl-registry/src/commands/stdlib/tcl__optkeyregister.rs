//! `tcl::OptKeyRegister` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::OptKeyRegister",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Register an option description list under a key for later use.",
            &["tcl::OptKeyRegister optlist ?key?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
