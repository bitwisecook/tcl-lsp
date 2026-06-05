//! `report_analysis_coverage` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_analysis_coverage ?-nosplit? ?-status_details untested?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_analysis_coverage",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report timing analysis coverage.",
            &["report_analysis_coverage ?-nosplit? ?-status_details untested?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
