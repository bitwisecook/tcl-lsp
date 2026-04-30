//! `check_timing_intent` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "check_timing_intent",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Verify timing intent completeness.",
            &["check_timing_intent ?-verbose?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
