//! `expect_background` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-re",
        takes_value: false,
        value_hint: "",
        detail: "Match as regular expression.",
        dialects: None,
    },
    OptionSpec {
        name: "-ex",
        takes_value: false,
        value_hint: "",
        detail: "Match as exact string.",
        dialects: None,
    },
    OptionSpec {
        name: "-gl",
        takes_value: false,
        value_hint: "",
        detail: "Match as glob (default).",
        dialects: None,
    },
    OptionSpec {
        name: "-nocase",
        takes_value: false,
        value_hint: "",
        detail: "Case-insensitive matching.",
        dialects: None,
    },
    OptionSpec {
        name: "-i",
        takes_value: true,
        value_hint: "spawn_id",
        detail: "Specify the spawn id.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "expect_background ?-opts? pattern body ?pattern body ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "expect_background",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Non-blocking expect: run pattern matching in the background.",
            synopsis: &["expect_background ?-opts? pattern body ?pattern body ...?"],
            snippet: "Like ``expect`` but does not block. Whenever new data arrives the patterns are tested and the matching body is executed.",
            source: "Expect expect_background(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
