//! `testgetassocdata` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testgetassocdata",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_GetAssocData.",
            &["testgetassocdata"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
