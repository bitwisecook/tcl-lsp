//! `get_report_panel_id` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "get_report_panel_id panel_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_report_panel_id",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Get the ID of a report panel.",
            &["get_report_panel_id panel_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
