//! `math::statistics::test-Dunnett` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::test-Dunnett",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Dunnett multiple comparison test.",
            &["math::statistics::test-Dunnett alpha control args"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
