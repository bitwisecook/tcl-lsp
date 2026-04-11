//! `get_global_assignment` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_global_assignment",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get a global project assignment value.",
            &["get_global_assignment -name name ?-section_id id?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
