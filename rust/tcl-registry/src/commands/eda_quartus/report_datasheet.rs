//! `report_datasheet` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_datasheet",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report I/O timing datasheet.",
            &["report_datasheet ?-file file? ?-panel_name name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
