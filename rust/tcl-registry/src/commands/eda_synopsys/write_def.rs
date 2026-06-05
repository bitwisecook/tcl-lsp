//! `write_def` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "write_def ?-version version? file_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "write_def",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Write a DEF file.",
            &["write_def ?-version version? file_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
