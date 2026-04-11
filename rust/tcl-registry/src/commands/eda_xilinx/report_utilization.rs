//! `report_utilization` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_utilization",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Report device utilization.", &["report_utilization ?-hierarchical? ?-hierarchical_depth n? ?-file file? ?-name name?"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
