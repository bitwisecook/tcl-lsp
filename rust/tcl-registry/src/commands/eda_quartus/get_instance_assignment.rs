//! `get_instance_assignment` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis:
        "get_instance_assignment -name name ?-to to? ?-from from? ?-entity entity? ?-section_id id?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_instance_assignment",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Get an instance assignment value.", &["get_instance_assignment -name name ?-to to? ?-from from? ?-entity entity? ?-section_id id?"], "F5")),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
