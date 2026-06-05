//! `read_def` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "read_def ?-add_def_only_objects all? file_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_def",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Read a DEF file.",
            &["read_def ?-add_def_only_objects all? file_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
