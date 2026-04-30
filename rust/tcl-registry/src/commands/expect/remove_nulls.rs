//! `remove_nulls` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "remove_nulls",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Control whether null bytes are removed from spawned process output.",
            &["remove_nulls ?-d | -i spawn_id? ?0 | 1?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
