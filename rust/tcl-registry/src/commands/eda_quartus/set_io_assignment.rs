//! `set_io_assignment` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "set_io_assignment -name name -to pin_name value",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_io_assignment",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set an I/O assignment.",
            &["set_io_assignment -name name -to pin_name value"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
