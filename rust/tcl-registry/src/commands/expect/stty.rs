//! `stty` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "stty ?args?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "stty",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set or query terminal modes (raw, echo, rows, columns, etc.).",
            &["stty ?args?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
