//! `create_floorplan` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[
    FormSpec { kind: FormKind::Default, synopsis: "create_floorplan ?-core_utilization util? ?-core_aspect_ratio ratio? ?-core_margins_by die|core? margins" },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_floorplan",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Create a floorplan.", &["create_floorplan ?-core_utilization util? ?-core_aspect_ratio ratio? ?-core_margins_by die|core? mar"], "F5")),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
