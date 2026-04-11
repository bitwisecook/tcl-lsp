//! `report_design` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_design",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report design summary.",
            &["report_design ?-nosplit? ?-verbose?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
