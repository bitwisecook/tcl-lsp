//! `timestamp` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "timestamp",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Return the current time or format a timestamp.",
            &["timestamp ?-seconds N? ?-format fmt? ?-gmt?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
