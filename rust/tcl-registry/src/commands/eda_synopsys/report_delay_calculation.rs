//! `report_delay_calculation` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_delay_calculation",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report delay calculation details.",
            &["report_delay_calculation ?-from from_pin? ?-to to_pin?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
