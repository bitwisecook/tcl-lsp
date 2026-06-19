//! `set_global_assignment` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "set_global_assignment -name name value ?-section_id id?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_global_assignment",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Set a global project assignment.",
            &["set_global_assignment -name name value ?-section_id id?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
