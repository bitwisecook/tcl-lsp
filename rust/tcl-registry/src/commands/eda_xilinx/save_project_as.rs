//! `save_project_as` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "save_project_as ?-force? project_name ?project_dir?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "save_project_as",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Save the project with a new name.",
            &["save_project_as ?-force? project_name ?project_dir?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
