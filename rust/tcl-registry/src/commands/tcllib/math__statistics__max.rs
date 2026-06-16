//! `math::statistics::max` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::max values",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::max",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return the maximum value.",
            synopsis: &["math::statistics::max values"],
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
