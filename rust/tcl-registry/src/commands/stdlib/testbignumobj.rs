//! `testbignumobj` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testbignumobj",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test bignum Tcl_Obj operations.",
            &["testbignumobj"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
