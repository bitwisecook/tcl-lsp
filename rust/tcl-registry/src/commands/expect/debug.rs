//! `debug` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-now",
    takes_value: false,
    value_hint: "",
    detail: "Enter debugger immediately.",
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "debug ?-now? ?0 | 1?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "debug",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enable or disable the Expect debugger.",
            &["debug ?-now? ?0 | 1?"],
            "F5",
        )),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
