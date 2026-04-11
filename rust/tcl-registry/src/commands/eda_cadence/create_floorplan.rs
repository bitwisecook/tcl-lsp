//! `create_floorplan` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_floorplan",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Create a floorplan.", &["create_floorplan ?-core_utilization util? ?-core_aspect_ratio ratio? ?-core_margins_by die|core? mar"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
