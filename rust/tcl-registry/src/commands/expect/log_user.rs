//! `log_user` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-info",
    takes_value: false,
    value_hint: "",
    detail: "Return current setting.",
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "log_user ?-info | 0 | 1?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "log_user",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Control whether send/expect output is logged to stdout.",
            &["log_user -info"],
            "F5",
        )),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
