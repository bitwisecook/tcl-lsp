//! `route_design` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "route_design",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Route the design.",
            &["route_design ?-global_detail?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
