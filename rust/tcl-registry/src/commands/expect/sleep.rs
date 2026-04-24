//! `sleep` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "sleep",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Pause execution for the specified number of seconds.",
            &["sleep seconds"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
