//! `update_timing` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "update_timing",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Update incremental timing.",
            &["update_timing ?-full?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
