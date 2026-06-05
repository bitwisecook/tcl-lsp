//! `report_ucp` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_ucp ?-file file? ?-panel_name name?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_ucp",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report unconstrained paths.",
            &["report_ucp ?-file file? ?-panel_name name?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
