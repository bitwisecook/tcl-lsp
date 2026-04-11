//! `tcl::OptParse` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::OptParse",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Parse a list of arguments according to an option description.",
            &["tcl::OptParse optlist arglist"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
