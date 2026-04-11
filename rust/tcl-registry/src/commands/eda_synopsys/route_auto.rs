//! `route_auto` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "route_auto",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Perform automatic routing.",
            &["route_auto ?-max_detail_route_iterations n?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
