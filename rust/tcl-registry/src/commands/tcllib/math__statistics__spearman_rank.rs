//! `math::statistics::spearman-rank` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::spearman-rank",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Spearman rank correlation.",
            &["math::statistics::spearman-rank sample_a sample_b"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
