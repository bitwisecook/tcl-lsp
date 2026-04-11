//! `math::statistics::quantiles` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::quantiles",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet::brief(
            "Compute quantiles of a list of values.",
            &["math::statistics::quantiles data confidences"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
