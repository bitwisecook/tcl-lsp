//! `remove_all_assignments` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[
    FormSpec { kind: FormKind::Default, synopsis: "remove_all_assignments ?-name name? ?-to to? ?-entity entity? ?-type type? ?-section_id id?" },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "remove_all_assignments",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Remove all assignments matching criteria.", &["remove_all_assignments ?-name name? ?-to to? ?-entity entity? ?-type type? ?-section_id id?"], "F5")),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
