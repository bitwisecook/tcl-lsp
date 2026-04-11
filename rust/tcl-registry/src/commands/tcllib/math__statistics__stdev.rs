//! `math::statistics::stdev` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::stdev",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Compute the standard deviation of a list of values.",
            &["math::statistics::stdev data"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
