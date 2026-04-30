//! `tcltest::testConstraint` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::testConstraint",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Get or set a named test constraint boolean.",
            &["tcltest::testConstraint constraint ?value?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
