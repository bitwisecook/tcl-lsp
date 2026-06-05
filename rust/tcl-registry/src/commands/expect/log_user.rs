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
        hover: Some(HoverSnippet {
            summary: "Control whether send/expect output is logged to stdout.",
            synopsis: &["log_user -info", "log_user 0|1"],
            snippet:
                "With ``1`` (default), output is sent to stdout. With ``0``, output is suppressed.",
            source: "Expect log_user(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
