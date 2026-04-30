//! `report_clock_utilization` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_clock_utilization",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report clock resource utilization.",
            &["report_clock_utilization ?-file file? ?-name name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
