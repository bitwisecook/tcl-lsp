//! `math::statistics::spearman-rank` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::spearman-rank sample_a sample_b",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::spearman-rank",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Spearman rank correlation.",
            synopsis: &["math::statistics::spearman-rank sample_a sample_b"],
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
