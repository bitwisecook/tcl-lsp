//! `report_clock_fmax_summary` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_clock_fmax_summary",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report maximum clock frequency summary.",
            &["report_clock_fmax_summary ?-file file? ?-panel_name name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
