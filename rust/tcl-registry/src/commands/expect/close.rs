//! `close` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-slave",
        takes_value: false,
        value_hint: "",
        detail: "Close the slave side of the pty.",
        dialects: None,
    },
    OptionSpec {
        name: "-i",
        takes_value: true,
        value_hint: "spawn_id",
        detail: "Close the specified spawn id.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "close ?-slave? ?-i spawn_id?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "close",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Close the connection to the current spawned process.",
            synopsis: &["close ?-slave? ?-i spawn_id?"],
            snippet: "",
            source: "Expect close(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
