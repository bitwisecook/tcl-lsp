//! `opt_design` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "opt_design",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Optimize the design (post-route).",
            &["opt_design ?-post_route? ?-setup? ?-hold? ?-drv?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
