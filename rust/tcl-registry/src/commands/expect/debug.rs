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
        hover: Some(HoverSnippet {
            summary: "Enable or disable the Expect debugger.",
            synopsis: &["debug ?-now? ?0 | 1?"],
            snippet: "",
            source: "Expect debug(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
