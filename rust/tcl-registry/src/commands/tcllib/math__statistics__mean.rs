//! `math::statistics::mean` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::mean",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Compute the arithmetic mean of a list of values.",
            &["math::statistics::mean data"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
