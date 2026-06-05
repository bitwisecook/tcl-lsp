//! `interact` command.
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
        name: "-input",
        takes_value: true,
        value_hint: "spawn_id",
        detail: "Specify input source.",
        dialects: None,
    },
    OptionSpec {
        name: "-output",
        takes_value: true,
        value_hint: "spawn_id",
        detail: "Specify output destination.",
        dialects: None,
    },
    OptionSpec {
        name: "-u",
        takes_value: true,
        value_hint: "spawn_id",
        detail: "Connect user to the specified process.",
        dialects: None,
    },
    OptionSpec {
        name: "-o",
        takes_value: false,
        value_hint: "",
        detail: "Apply to output.",
        dialects: None,
    },
    OptionSpec {
        name: "-i",
        takes_value: true,
        value_hint: "spawn_id",
        detail: "Specify spawn id.",
        dialects: None,
    },
    OptionSpec {
        name: "-echo",
        takes_value: false,
        value_hint: "",
        detail: "Echo characters.",
        dialects: None,
    },
    OptionSpec {
        name: "-nobuffer",
        takes_value: false,
        value_hint: "",
        detail: "Do not buffer input.",
        dialects: None,
    },
    OptionSpec {
        name: "-f",
        takes_value: false,
        value_hint: "",
        detail: "Force — do not flush.",
        dialects: None,
    },
    OptionSpec {
        name: "-F",
        takes_value: false,
        value_hint: "",
        detail: "Force — flush.",
        dialects: None,
    },
    OptionSpec {
        name: "-reset",
        takes_value: false,
        value_hint: "",
        detail: "Reset terminal modes.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "interact ?-opts? ?string body ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "interact",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Give control of the current process to the user for interactive use.",
            &["interact ?-opts? ?string body ...?"],
            "F5",
        )),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
