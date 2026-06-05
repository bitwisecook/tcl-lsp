//! `send_log` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "--",
    takes_value: false,
    value_hint: "",
    detail: "End of options.",
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "send_log ?--? string",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "send_log",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Send a string to the log file only (not to the process or user).",
            &["send_log ?--? string"],
            "F5",
        )),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
