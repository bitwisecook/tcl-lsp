//! `testlongsize` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testlongsize",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report sizeof(long) (9.0+).",
            &["testlongsize"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
