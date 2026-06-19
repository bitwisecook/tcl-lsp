//! `initialize_floorplan` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "initialize_floorplan ?-core_utilization util? ?-core_offset offset?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "initialize_floorplan",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Initialize the floorplan from constraints.",
            &["initialize_floorplan ?-core_utilization util? ?-core_offset offset?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
