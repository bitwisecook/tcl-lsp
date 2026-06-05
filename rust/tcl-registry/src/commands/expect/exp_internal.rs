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
hover: Some(HoverSnippet {
            summary: "Control Expect internal diagnostic output.",
            synopsis: &["exp_internal ?-f file? 0|1"],
            snippet: "With ``1``, Expect prints diagnostic information about pattern matching and other internal activity. Useful for debugging scripts.",
            source: "Expect exp_internal(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
