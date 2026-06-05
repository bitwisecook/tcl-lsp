//! `strace` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "strace level",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "strace",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Trace Expect internal statements at the given detail level.",
            &["strace level"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
