//! `testlistrep` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testlistrep",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test list internal representation (9.0+).",
            &["testlistrep"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
