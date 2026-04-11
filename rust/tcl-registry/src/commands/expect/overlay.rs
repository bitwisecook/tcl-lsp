//! `overlay` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "overlay",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Replace the Expect process with another program (exec-style).",
            &["overlay ?-# spawn_id ...? program ?args ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
