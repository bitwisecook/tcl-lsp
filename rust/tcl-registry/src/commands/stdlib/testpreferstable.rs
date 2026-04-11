//! `testpreferstable` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testpreferstable",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test TIP 268 stable-vs-latest preference (9.0+).",
            &["testpreferstable"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
