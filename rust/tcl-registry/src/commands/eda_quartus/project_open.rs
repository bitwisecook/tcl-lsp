//! `project_open` command.
use crate::prelude::*;
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
        ..CommandSpec::DEFAULT
    }
}
