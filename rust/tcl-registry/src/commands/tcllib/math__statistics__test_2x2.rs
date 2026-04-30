//! `math::statistics::test-2x2` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::test-2x2",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(4),
        hover: Some(HoverSnippet::brief(
            "Analyse a 2x2 contingency table.",
            &["math::statistics::test-2x2 a b c d"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
