//! `create_project` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_project",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create a new Vivado project.",
            &["create_project ?-force? ?-part part? ?-in_memory? project_name ?project_dir?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
