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
        hover: Some(HoverSnippet {
            summary: "Pause execution for the specified number of seconds.",
            synopsis: &["sleep seconds"],
            snippet: "Accepts integer or decimal values (e.g. ``sleep 0.5``).",
            source: "Expect sleep(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
