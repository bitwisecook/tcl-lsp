//! `expect_tty` command.
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
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "expect_tty ?-opts? pattern body ?pattern body ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "expect_tty",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Expect input from the controlling terminal (tty).",
            synopsis: &["expect_tty ?-opts? pattern body ?pattern body ...?"],
            snippet: "",
            source: "Expect expect_tty(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
