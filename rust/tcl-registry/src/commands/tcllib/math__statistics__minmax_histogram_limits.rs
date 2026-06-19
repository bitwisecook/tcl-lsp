//! `math::statistics::minmax-histogram-limits` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::minmax-histogram-limits min max ?number?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::minmax-histogram-limits",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet {
            summary: "Compute histogram limits from min and max.",
            synopsis: &["math::statistics::minmax-histogram-limits min max ?number?"],
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
