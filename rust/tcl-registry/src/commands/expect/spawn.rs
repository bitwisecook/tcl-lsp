//! `spawn` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "spawn",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Start a new process and prepare it for interaction.",
            &["spawn ?-option ...? program ?args ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
