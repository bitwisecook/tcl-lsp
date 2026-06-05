//! `project_open` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "project_open ?-revision rev? ?-current_revision? ?-force? project_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "project_open",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Open an existing Quartus project.",
            &["project_open ?-revision rev? ?-current_revision? ?-force? project_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
