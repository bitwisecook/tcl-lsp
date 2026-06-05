//! `project_exists` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "project_exists project_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "project_exists",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Check whether a Quartus project exists.",
            &["project_exists project_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
