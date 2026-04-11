//! `report_area` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_area",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report design area.",
            &["report_area ?-physical? ?-verbose?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
