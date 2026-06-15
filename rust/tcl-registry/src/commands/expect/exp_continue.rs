//! `exp_continue` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-continue_timer",
    takes_value: false,
    value_hint: "",
    detail: "Do not restart the timeout timer.",
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "exp_continue ?-continue_timer?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exp_continue",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary: "Continue matching within an expect body instead of returning.",
            synopsis: &["exp_continue ?-continue_timer?"],
            snippet: "Used inside an ``expect`` body to re-enter the pattern matching loop. With ``-continue_timer``, the timeout timer is not restarted.",
            source: "Expect exp_continue(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
