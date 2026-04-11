//! `report_analysis_coverage` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_analysis_coverage",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report timing analysis coverage.",
            &["report_analysis_coverage"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
