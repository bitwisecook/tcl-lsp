//! `add_filler` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "add_filler",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Add filler cells.",
            &["add_filler ?-cell cell_list? ?-prefix prefix?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
