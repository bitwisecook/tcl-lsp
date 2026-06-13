//! `expect` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-re",
        takes_value: false,
        value_hint: "",
        detail: "Match pattern as a Tcl regular expression.",
        dialects: None,
    },
    OptionSpec {
        name: "-ex",
        takes_value: false,
        value_hint: "",
        detail: "Match pattern as an exact string.",
        dialects: None,
    },
    OptionSpec {
        name: "-gl",
        takes_value: false,
        value_hint: "",
        detail: "Match pattern as a glob (default).",
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
        name: "-timeout",
        takes_value: true,
        value_hint: "seconds",
        detail: "Override the timeout for this expect.",
        dialects: None,
    },
    OptionSpec {
        name: "-i",
        takes_value: true,
        value_hint: "spawn_id",
        detail: "Specify the spawn id to expect from.",
        dialects: None,
    },
    OptionSpec {
        name: "-indices",
        takes_value: false,
        value_hint: "",
        detail: "Store match indices in expect_out.",
        dialects: None,
    },
    OptionSpec {
        name: "-notransfer",
        takes_value: false,
        value_hint: "",
        detail: "Do not consume matched output.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "expect ?-opts? pattern body ?pattern body ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "expect",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Wait for output matching a pattern from a spawned process.",
            synopsis: &["expect ?-opts? pattern body ?pattern body ...?", "expect -re {regexp} { actions }", "expect timeout { timeout_actions }", "expect eof { eof_actions }"],
            snippet: "Waits until one of the patterns matches the output of the current spawned process, then executes the corresponding body. Special patterns: ``timeout``, ``eof``, ``default``, ``full_buffer``, ``null``.",
            source: "Expect expect(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
