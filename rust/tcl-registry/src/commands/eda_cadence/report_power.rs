//! `report_power` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_power",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report power consumption.",
            &["report_power ?-leakage? ?-dynamic? ?-view view_name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
