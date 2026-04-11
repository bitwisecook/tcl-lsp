//! `place_opt_design` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "place_opt_design",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Perform placement and optimization.",
            &["place_opt_design ?-effort effort? ?-incremental?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
