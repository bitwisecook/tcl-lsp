//! `report_clock_gating` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_clock_gating",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report clock gating statistics.",
            &["report_clock_gating ?-nosplit? ?-verbose?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
