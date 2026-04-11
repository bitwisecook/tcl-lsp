//! `report_reference` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_reference",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report library cell references.",
            &["report_reference ?-nosplit? ?-hierarchy?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
