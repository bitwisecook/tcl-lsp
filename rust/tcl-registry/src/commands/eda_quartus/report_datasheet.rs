//! `report_datasheet` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_datasheet ?-file file? ?-panel_name name?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_datasheet",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report I/O timing datasheet.",
            &["report_datasheet ?-file file? ?-panel_name name?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
