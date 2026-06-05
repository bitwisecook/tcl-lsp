//! `send_user` command.
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
    synopsis: "send_user ?-flags? string",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "send_user",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Send a string to the user (standard output).",
            &["send_user ?-flags? string"],
            "F5",
        )),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
