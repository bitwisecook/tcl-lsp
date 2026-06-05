//! `report_clock_fmax_summary` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_clock_fmax_summary ?-file file? ?-panel_name name?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_clock_fmax_summary",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report maximum clock frequency summary.",
            &["report_clock_fmax_summary ?-file file? ?-panel_name name?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
