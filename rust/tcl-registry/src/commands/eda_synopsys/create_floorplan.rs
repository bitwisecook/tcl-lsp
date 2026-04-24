//! `create_floorplan` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_floorplan",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Create an initial floorplan.", &["create_floorplan ?-core_utilization util? ?-core_aspect_ratio ratio? ?-left_io2core dist? ?-bottom_i"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
