//! `testwrongnumargs` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testwrongnumargs",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_WrongNumArgs.",
            &["testwrongnumargs"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
