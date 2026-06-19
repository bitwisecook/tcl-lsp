//! `math::statistics::test-normal` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::test-normal data significance",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::test-normal",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Test for normal distribution.",
            synopsis: &["math::statistics::test-normal data significance"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("math::statistics"),
        required_package: Some("math::statistics"),
        ..CommandSpec::DEFAULT
    }
}
