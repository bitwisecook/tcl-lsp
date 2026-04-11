//! `math::statistics::max` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::max",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the maximum value.",
            &["math::statistics::max values"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
