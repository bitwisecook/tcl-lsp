//! `math::statistics::test-Duckworth` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::test-Duckworth list1 list2 significance",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::test-Duckworth",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(3),
        hover: Some(HoverSnippet {
            summary: "Perform Duckworth test.",
            synopsis: &["math::statistics::test-Duckworth list1 list2 significance"],
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
