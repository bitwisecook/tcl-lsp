//! `tcl::OptProcArgGiven` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::OptProcArgGiven",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return 1 if the named option was explicitly given, 0 otherwise.",
            &["tcl::OptProcArgGiven name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
