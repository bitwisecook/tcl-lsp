//! `get_all_assignments` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "get_all_assignments ?-name name? ?-to to? ?-entity entity? ?-type type? ?-section_id id?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_all_assignments",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get all assignments matching criteria.",
            &[
                "get_all_assignments ?-name name? ?-to to? ?-entity entity? ?-type type? ?-section_id id?",
            ],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
