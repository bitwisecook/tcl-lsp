//! `time_design` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "time_design",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Perform timing analysis.",
            &["time_design ?-pre_cts? ?-post_cts? ?-post_route? ?-hold? ?-report_prefix prefix?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
