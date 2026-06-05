//! `report_timing` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[
    FormSpec { kind: FormKind::Default, synopsis: "report_timing ?-from from? ?-to to? ?-through through? ?-delay_type type? ?-max_paths n? ?-nworst n? ?-sort_by attr? ?-file file? ?-name name?" },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_timing",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Report timing paths.", &["report_timing ?-from from? ?-to to? ?-through through? ?-delay_type type? ?-max_paths n? ?-nworst n?"], "F5")),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
