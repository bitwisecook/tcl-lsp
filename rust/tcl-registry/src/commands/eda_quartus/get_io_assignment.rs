//! `get_io_assignment` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "get_io_assignment -name name -to pin_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_io_assignment",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get an I/O assignment value.",
            &["get_io_assignment -name name -to pin_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
