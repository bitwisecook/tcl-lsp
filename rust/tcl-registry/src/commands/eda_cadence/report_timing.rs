//! `report_timing` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_timing",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report timing paths.",
            &["report_timing ?-from from? ?-to to? ?-through through? ?-max_paths n? ?-nworst n?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
