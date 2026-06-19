//! `exit` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-onexit",
        takes_value: true,
        value_hint: "command",
        detail: "Register a handler to run at exit.",
        dialects: None,
    },
    OptionSpec {
        name: "-noexit",
        takes_value: false,
        value_hint: "",
        detail: "Prepare for exit without exiting.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "exit ?-onexit command | -noexit? ?status?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exit",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Exit Expect, optionally running an onexit handler.",
            synopsis: &["exit ?-onexit command? ?status?", "exit ?-noexit? ?status?"],
            snippet: "With ``-onexit``, registers a handler to run at exit. With ``-noexit``, prepares for exit but does not actually exit (useful for cleaning up in libraries).",
            source: "Expect exit(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
