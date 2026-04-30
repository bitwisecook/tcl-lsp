//! `math::statistics::test-Kruskal-Wallis` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::test-Kruskal-Wallis",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Kruskal-Wallis rank test.",
            &["math::statistics::test-Kruskal-Wallis confidence args"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
