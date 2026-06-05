//! `set_location_assignment` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "set_location_assignment -to pin_name location",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_location_assignment",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Set a location assignment for a pin.",
            &["set_location_assignment -to pin_name location"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
