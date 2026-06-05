//! `timestamp` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-seconds",
        takes_value: true,
        value_hint: "N",
        detail: "Specify epoch seconds instead of current time.",
        dialects: None,
    },
    OptionSpec {
        name: "-format",
        takes_value: true,
        value_hint: "fmt",
        detail: "strftime-style format string.",
        dialects: None,
    },
    OptionSpec {
        name: "-gmt",
        takes_value: false,
        value_hint: "",
        detail: "Use GMT instead of local time.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "timestamp ?-seconds N? ?-format fmt? ?-gmt?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "timestamp",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Return the current time or format a timestamp.",
            synopsis: &["timestamp ?-seconds N? ?-format fmt? ?-gmt?", "timestamp"],
            snippet: "",
            source: "Expect timestamp(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
