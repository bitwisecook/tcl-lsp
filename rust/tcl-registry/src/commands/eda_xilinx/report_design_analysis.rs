//! `report_design_analysis` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis:
        "report_design_analysis ?-timing? ?-logic_level_distribution? ?-file file? ?-name name?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_design_analysis",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Report design analysis metrics.", &["report_design_analysis ?-timing? ?-logic_level_distribution? ?-file file? ?-name name?"], "F5")),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
