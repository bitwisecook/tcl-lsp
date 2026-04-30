//! `remove_all_assignments` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "remove_all_assignments",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Remove all assignments matching criteria.", &["remove_all_assignments ?-name name? ?-to to? ?-entity entity? ?-type type? ?-section_id id?"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
