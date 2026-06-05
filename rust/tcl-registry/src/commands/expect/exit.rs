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
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Exit Expect, optionally running an onexit handler.",
            &["exit ?-onexit command? ?status?"],
            "F5",
        )),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
