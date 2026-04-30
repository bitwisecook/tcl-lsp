//! `get_all_assignments` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_all_assignments",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Get all assignments matching criteria.", &["get_all_assignments ?-name name? ?-to to? ?-entity entity? ?-type type? ?-section_id id?"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
