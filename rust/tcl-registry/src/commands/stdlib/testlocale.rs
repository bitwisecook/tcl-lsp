//! `testlocale` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testlocale",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test locale operations.",
            &["testlocale"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
