//! `math::statistics::test-Tukey-range` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::test-Tukey-range alpha args",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::test-Tukey-range",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Tukey range test.",
            synopsis: &["math::statistics::test-Tukey-range alpha args"],
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
