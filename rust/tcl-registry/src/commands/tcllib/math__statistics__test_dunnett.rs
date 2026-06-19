//! `math::statistics::test-Dunnett` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::test-Dunnett alpha control args",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::test-Dunnett",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet {
            summary: "Dunnett multiple comparison test.",
            synopsis: &["math::statistics::test-Dunnett alpha control args"],
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
