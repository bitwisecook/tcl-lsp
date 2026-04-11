//! `math::statistics::test-normal` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::test-normal",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Test for normal distribution.",
            &["math::statistics::test-normal data significance"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
