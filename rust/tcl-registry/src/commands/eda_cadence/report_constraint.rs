//! `report_constraint` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_constraint",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report constraint violations.",
            &["report_constraint ?-all_violators? ?-drv_violation_type type?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
