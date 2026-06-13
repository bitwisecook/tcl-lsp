//! `create_delay_corner` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "create_delay_corner -name name ?-library_set lib_set? ?-rc_corner rc_corner?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_delay_corner",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create a delay corner for MMMC.",
            &["create_delay_corner -name name ?-library_set lib_set? ?-rc_corner rc_corner?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
