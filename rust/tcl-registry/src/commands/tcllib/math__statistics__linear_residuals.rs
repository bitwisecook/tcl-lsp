//! `math::statistics::linear-residuals` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::linear-residuals",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet::brief(
            "Compute residuals of a linear model.",
            &["math::statistics::linear-residuals xdata ydata ?intercept?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
