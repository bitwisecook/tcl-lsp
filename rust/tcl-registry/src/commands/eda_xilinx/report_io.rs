//! `report_io` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_io",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report I/O port assignments.",
            &["report_io ?-file file? ?-name name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
