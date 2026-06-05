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
        hover: Some(HoverSnippet {
            summary: "Set or query terminal modes (raw, echo, rows, columns, etc.).",
            synopsis: &["stty ?args?", "stty raw -echo", "stty -raw echo"],
            snippet: "",
            source: "Expect stty(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
