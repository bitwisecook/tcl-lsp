//! `send_tty` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-raw",
        takes_value: false,
        value_hint: "",
        detail: "Send without translation.",
        dialects: None,
    },
    OptionSpec {
        name: "--",
        takes_value: false,
        value_hint: "",
        detail: "End of options.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "send_tty ?-flags? string",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "send_tty",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Send a string to the controlling terminal (tty).",
            synopsis: &["send_tty ?-flags? string"],
            snippet: "",
            source: "Expect send_tty(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
