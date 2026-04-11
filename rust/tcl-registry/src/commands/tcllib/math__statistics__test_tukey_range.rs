//! `math::statistics::test-Tukey-range` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::test-Tukey-range",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Tukey range test.",
            &["math::statistics::test-Tukey-range alpha args"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
