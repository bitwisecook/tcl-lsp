//! `report_timing` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_timing",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Report timing paths.", &["report_timing ?-from from? ?-to to? ?-through through? ?-delay_type type? ?-max_paths n? ?-nworst n?"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
