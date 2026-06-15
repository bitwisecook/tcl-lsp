//! `report_timing` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_timing ?-from from_list? ?-through through_list? ?-to to_list? ?-delay_type type? ?-max_paths n? ?-nworst n? ?-sort_by attr?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_timing",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report timing paths.",
            &[
                "report_timing ?-from from_list? ?-through through_list? ?-to to_list? ?-delay_type type? ?-max_paths",
            ],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
