//! `math::statistics::min` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::min",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the minimum value.",
            &["math::statistics::min values"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
