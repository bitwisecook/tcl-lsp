//! `report_clock_networks` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_clock_networks",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report clock network topology.",
            &["report_clock_networks ?-file file? ?-name name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
