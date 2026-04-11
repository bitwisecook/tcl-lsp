//! `project_exists` command.
use crate::prelude::*;
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
        ..CommandSpec::DEFAULT
    }
}
