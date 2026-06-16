//! `math::statistics::stdev` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::stdev data",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::stdev",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Compute the standard deviation of a list of values.",
            synopsis: &["math::statistics::stdev data"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "The standard deviation.",
        }),
        forms: FORMS,
        tcllib_package: Some("math::statistics"),
        required_package: Some("math::statistics"),
        ..CommandSpec::DEFAULT
    }
}
