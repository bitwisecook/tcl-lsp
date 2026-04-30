//! `add_well_tap` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "add_well_tap",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Add well-tap cells.",
            &["add_well_tap ?-cell cell? ?-cell_interval interval?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
