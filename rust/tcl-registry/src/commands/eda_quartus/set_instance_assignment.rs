//! `set_instance_assignment` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[
    FormSpec { kind: FormKind::Default, synopsis: "set_instance_assignment -name name ?-to to? ?-from from? ?-entity entity? ?-section_id id? value" },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_instance_assignment",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Set an assignment on a specific instance.", &["set_instance_assignment -name name ?-to to? ?-from from? ?-entity entity? ?-section_id id? value"], "F5")),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
