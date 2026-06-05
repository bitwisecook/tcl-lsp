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
        hover: Some(HoverSnippet {
            summary: "Trace Expect internal statements at the given detail level.",
            synopsis: &["strace level"],
            snippet: "",
            source: "Expect strace(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
