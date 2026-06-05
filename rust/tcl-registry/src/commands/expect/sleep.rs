//! `sleep` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "sleep seconds",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "sleep",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Pause execution for the specified number of seconds.",
            &["sleep seconds"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
