//! `report_analysis_coverage` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_analysis_coverage",
}];

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
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
