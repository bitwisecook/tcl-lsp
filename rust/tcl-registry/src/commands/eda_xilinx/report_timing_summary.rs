//! `report_timing_summary` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_timing_summary",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Report comprehensive timing summary.", &["report_timing_summary ?-delay_type type? ?-max_paths n? ?-check_timing_verbose? ?-file file? ?-name "], "F5")),
        ..CommandSpec::DEFAULT
    }
}
