//! `report_congestion` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_congestion",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report routing congestion.",
            &["report_congestion ?-nosplit?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
