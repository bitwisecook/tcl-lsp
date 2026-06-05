//! `exp_internal` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-f",
    takes_value: true,
    value_hint: "file",
    detail: "Log diagnostics to the specified file.",
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "exp_internal ?-f file? 0|1",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exp_internal",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Control Expect internal diagnostic output.",
            &["exp_internal ?-f file? 0|1"],
            "F5",
        )),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
