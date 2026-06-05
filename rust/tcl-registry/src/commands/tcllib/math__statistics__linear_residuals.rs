//! `math::statistics::linear-residuals` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::linear-residuals xdata ydata ?intercept?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::linear-residuals",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet {
            summary: "Compute residuals of a linear model.",
            synopsis: &["math::statistics::linear-residuals xdata ydata ?intercept?"],
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
