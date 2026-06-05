//! `write_sdc` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "write_sdc > file",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "write_sdc",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Write SDC constraints.",
            &["write_sdc > file"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
