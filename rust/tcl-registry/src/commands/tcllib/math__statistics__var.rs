//! `math::statistics::var` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::var",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Compute the variance of a list of values.",
            &["math::statistics::var data"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
