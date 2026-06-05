//! `overlay` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "overlay ?-# spawn_id ...? program ?args ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "overlay",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Replace the Expect process with another program (exec-style).",
            synopsis: &["overlay ?-# spawn_id ...? program ?args ...?"],
            snippet: "",
            source: "Expect overlay(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
