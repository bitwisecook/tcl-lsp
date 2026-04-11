//! `math::statistics::print-2x2` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::print-2x2",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(4),
        hover: Some(HoverSnippet::brief(
            "Format a 2x2 contingency table.",
            &["math::statistics::print-2x2 a b c d"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
