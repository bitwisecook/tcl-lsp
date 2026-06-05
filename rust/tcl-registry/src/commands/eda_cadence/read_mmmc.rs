//! `read_mmmc` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "read_mmmc file_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_mmmc",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Read Multi-Mode Multi-Corner (MMMC) configuration.",
            &["read_mmmc file_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
