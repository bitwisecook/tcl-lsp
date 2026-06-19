//! `project_close` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "project_close ?-dont_export_assignments?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "project_close",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Close the current Quartus project.",
            &["project_close ?-dont_export_assignments?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
