//! `report_timing` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_timing ?-from from_list? ?-to to_list? ?-through through_list? ?-setup | -hold? ?-npaths n? ?-detail detail? ?-panel_name name? ?-file file?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_timing",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report timing paths.",
            &[
                "report_timing ?-from from_list? ?-to to_list? ?-through through_list? ?-setup | -hold? ?-npaths n? ?",
            ],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
