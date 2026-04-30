//! `math::statistics::analyse-Kruskal-Wallis` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::analyse-Kruskal-Wallis",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Analyse Kruskal-Wallis results.",
            &["math::statistics::analyse-Kruskal-Wallis args"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
