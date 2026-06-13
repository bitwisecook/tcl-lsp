//! `report_power` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_power ?-leakage? ?-dynamic? ?-view view_name?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_power",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report power consumption.",
            &["report_power ?-leakage? ?-dynamic? ?-view view_name?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
