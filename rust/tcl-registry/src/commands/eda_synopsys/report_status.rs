//! `report_status` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_status",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report verification status.",
            &["report_status ?-verbose?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
