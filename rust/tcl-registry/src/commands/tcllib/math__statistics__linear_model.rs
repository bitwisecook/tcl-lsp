//! `math::statistics::linear-model` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::linear-model xdata ydata ?intercept?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::linear-model",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet {
            summary: "Fit a linear regression model.",
            synopsis: &["math::statistics::linear-model xdata ydata ?intercept?"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
