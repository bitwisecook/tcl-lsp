//! `wait` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-i",
        takes_value: true,
        value_hint: "spawn_id",
        detail: "Wait for the specified spawn id.",
        dialects: None,
    },
    OptionSpec {
        name: "-nowait",
        takes_value: false,
        value_hint: "",
        detail: "Non-blocking wait.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "wait ?-i spawn_id? ?-nowait?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "wait",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Wait for a spawned process to terminate.",
            synopsis: &["wait ?-i spawn_id? ?-nowait?"],
            snippet: "Returns a list of four integers: pid, spawn id, OS error, and exit status (or -1 0 0 status on success).",
            source: "Expect wait(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
