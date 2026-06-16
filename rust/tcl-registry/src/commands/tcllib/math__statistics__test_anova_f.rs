//! `math::statistics::test-anova-F` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::test-anova-F alpha args",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::test-anova-F",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "One-way ANOVA F-test.",
            synopsis: &["math::statistics::test-anova-F alpha args"],
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
