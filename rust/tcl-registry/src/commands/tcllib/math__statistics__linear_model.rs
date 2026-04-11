//! `math::statistics::linear-model` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::linear-model",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet::brief(
            "Fit a linear regression model.",
            &["math::statistics::linear-model xdata ydata ?intercept?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
