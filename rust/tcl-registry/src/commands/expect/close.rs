//! `close` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "close",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Close the connection to the current spawned process.",
            &["close ?-slave? ?-i spawn_id?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
