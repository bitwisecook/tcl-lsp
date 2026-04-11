//! `report_hierarchy` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_hierarchy",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report design hierarchy.",
            &["report_hierarchy ?-nosplit? ?-full?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
