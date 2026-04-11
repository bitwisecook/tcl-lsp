//! `set_location_assignment` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_location_assignment",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Set a location assignment for a pin.",
            &["set_location_assignment -to pin_name location"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
