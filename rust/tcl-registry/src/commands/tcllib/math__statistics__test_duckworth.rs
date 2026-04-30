//! `math::statistics::test-Duckworth` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::test-Duckworth",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(3),
        hover: Some(HoverSnippet::brief(
            "Perform Duckworth test.",
            &["math::statistics::test-Duckworth list1 list2 significance"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
