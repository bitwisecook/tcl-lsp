//! `report_area` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_area ?-physical? ?-verbose?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_area",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report design area.",
            &["report_area ?-physical? ?-verbose?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
