//! `report_dp` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_dp",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report datapath resources.",
            &["report_dp ?-all?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
