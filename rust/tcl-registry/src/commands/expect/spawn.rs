//! `spawn` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-noecho",
        takes_value: false,
        value_hint: "",
        detail: "Suppress echoing of the command.",
        dialects: None,
    },
    OptionSpec {
        name: "-console",
        takes_value: false,
        value_hint: "",
        detail: "Redirect console output to spawn.",
        dialects: None,
    },
    OptionSpec {
        name: "-ignore",
        takes_value: true,
        value_hint: "signal",
        detail: "Ignore the named signal in the spawned process.",
        dialects: None,
    },
    OptionSpec {
        name: "-leaveopen",
        takes_value: false,
        value_hint: "",
        detail: "Leave the file descriptor open.",
        dialects: None,
    },
    OptionSpec {
        name: "-pty",
        takes_value: false,
        value_hint: "",
        detail: "Open a pty for the process.",
        dialects: None,
    },
    OptionSpec {
        name: "-nottycopy",
        takes_value: false,
        value_hint: "",
        detail: "Do not copy tty modes.",
        dialects: None,
    },
    OptionSpec {
        name: "-nottyinit",
        takes_value: false,
        value_hint: "",
        detail: "Do not initialise the tty.",
        dialects: None,
    },
    OptionSpec {
        name: "-open",
        takes_value: true,
        value_hint: "fileId",
        detail: "Use an already-open file id.",
        dialects: None,
    },
    OptionSpec {
        name: "-trap",
        takes_value: false,
        value_hint: "",
        detail: "Enable signal trapping.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "spawn ?-option ...? program ?args ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "spawn",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Start a new process and prepare it for interaction.",
            &["spawn ?-option ...? program ?args ...?"],
            "F5",
        )),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
