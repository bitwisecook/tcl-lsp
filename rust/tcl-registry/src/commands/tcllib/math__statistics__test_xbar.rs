//! `math::statistics::test-xbar` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::test-xbar",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Test data against X-bar control limits.",
            &["math::statistics::test-xbar control data"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
