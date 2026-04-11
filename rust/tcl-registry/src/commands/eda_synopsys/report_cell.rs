//! `report_cell` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_cell",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report cell-level information.",
            &["report_cell ?-nosplit? ?-connections? ?cell_list?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
