//! `current_project` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "current_project",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set the current project.",
            &["current_project ?project_name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
