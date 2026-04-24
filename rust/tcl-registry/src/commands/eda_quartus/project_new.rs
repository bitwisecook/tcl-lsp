//! `project_new` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "project_new",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief("Create a new Quartus project.", &["project_new ?-overwrite? ?-revision rev? ?-part part? ?-family family? project_name"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
