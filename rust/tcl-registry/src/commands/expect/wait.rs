//! `wait` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "wait",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Wait for a spawned process to terminate.",
            &["wait ?-i spawn_id? ?-nowait?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
