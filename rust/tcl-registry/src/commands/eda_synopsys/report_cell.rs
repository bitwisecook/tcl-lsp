//! `report_cell` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_cell ?-nosplit? ?-connections? ?cell_list?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_cell",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report cell-level information.",
            &["report_cell ?-nosplit? ?-connections? ?cell_list?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
