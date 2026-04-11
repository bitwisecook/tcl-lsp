//! `report_power` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_power",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report power consumption.",
            &["report_power ?-file file? ?-name name? ?-advisory?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
