//! `report_bottleneck` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_bottleneck",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report timing bottleneck analysis.",
            &["report_bottleneck ?-nosplit? ?-max_cells n?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
