//! `report_constraint` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_constraint",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report design constraints.",
            &["report_constraint ?-all_violators? ?-max_delay? ?-min_delay?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
