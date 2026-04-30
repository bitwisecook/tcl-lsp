//! `math::statistics::median` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::median",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Compute the median of a list of values.",
            &["math::statistics::median data"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
