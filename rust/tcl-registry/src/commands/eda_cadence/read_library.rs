//! `read_library` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "read_library ?-liberty? ?-lef? file_list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_library",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Read technology library files.",
            &["read_library ?-liberty? ?-lef? file_list"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
