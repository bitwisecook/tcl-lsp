//! `create_floorplan` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "create_floorplan ?-core_utilization util? ?-core_aspect_ratio ratio? ?-left_io2core dist? ?-bottom_io2core dist? ?-right_io2core dist? ?-top_io2core dist?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_floorplan",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Create an initial floorplan.",
            &[
                "create_floorplan ?-core_utilization util? ?-core_aspect_ratio ratio? ?-left_io2core dist? ?-bottom_i",
            ],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
