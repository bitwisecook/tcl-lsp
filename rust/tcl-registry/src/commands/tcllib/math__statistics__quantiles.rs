//! `math::statistics::quantiles` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::quantiles data confidences",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::quantiles",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet {
            summary: "Compute quantiles of a list of values.",
            synopsis: &["math::statistics::quantiles data confidences"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "A list of quantile values.",
        }),
        forms: FORMS,
        tcllib_package: Some("math::statistics"),
        required_package: Some("math::statistics"),
        ..CommandSpec::DEFAULT
    }
}
