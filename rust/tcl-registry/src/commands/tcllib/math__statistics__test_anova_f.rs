//! `math::statistics::test-anova-F` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::test-anova-F",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "One-way ANOVA F-test.",
            &["math::statistics::test-anova-F alpha args"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
