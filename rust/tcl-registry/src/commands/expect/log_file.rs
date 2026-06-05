//! `log_file` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-a",
        takes_value: false,
        value_hint: "",
        detail: "Append to existing log file.",
        dialects: None,
    },
    OptionSpec {
        name: "-noappend",
        takes_value: false,
        value_hint: "",
        detail: "Overwrite existing log file.",
        dialects: None,
    },
    OptionSpec {
        name: "-open",
        takes_value: true,
        value_hint: "fileId",
        detail: "Log to an already-open Tcl file id.",
        dialects: None,
    },
    OptionSpec {
        name: "-leaveopen",
        takes_value: false,
        value_hint: "",
        detail: "Leave the file open on close.",
        dialects: None,
    },
    OptionSpec {
        name: "-info",
        takes_value: false,
        value_hint: "",
        detail: "Return current log file settings.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "log_file ?-option ...? ?file?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "log_file",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Control logging of session output to a file.",
            synopsis: &["log_file ?-option ...? ?file?", "log_file -info"],
            snippet: "",
            source: "Expect log_file(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
