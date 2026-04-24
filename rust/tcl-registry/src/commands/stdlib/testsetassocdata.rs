//! `testsetassocdata` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testsetassocdata",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_SetAssocData.",
            &["testsetassocdata"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
