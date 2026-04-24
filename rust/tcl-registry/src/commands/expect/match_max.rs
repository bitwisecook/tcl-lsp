//! `match_max` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "match_max",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set or query the maximum match buffer size.",
            &["match_max ?-d | -i spawn_id? ?size?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
