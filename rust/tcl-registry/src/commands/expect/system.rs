//! `system` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "system args",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "system",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Execute a command string via the system shell (/bin/sh -c).",
            synopsis: &["system args"],
            snippet: "",
            source: "Expect system(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
