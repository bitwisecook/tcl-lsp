//! `report_methodology` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_methodology",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Run and report methodology checks.",
            &["report_methodology ?-file file? ?-name name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
