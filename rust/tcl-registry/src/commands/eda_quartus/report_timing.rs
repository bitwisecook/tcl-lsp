//! `report_timing` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_timing",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Report timing paths.", &["report_timing ?-from from_list? ?-to to_list? ?-through through_list? ?-setup | -hold? ?-npaths n? ?"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
