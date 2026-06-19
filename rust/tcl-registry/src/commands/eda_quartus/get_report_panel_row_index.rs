//! `get_report_panel_row_index` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "get_report_panel_row_index -name panel_name row_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_report_panel_row_index",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get the row index of matching data.",
            &["get_report_panel_row_index -name panel_name row_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
