//! `save_project_as` command.
use crate::prelude::*;
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
        ..CommandSpec::DEFAULT
    }
}
