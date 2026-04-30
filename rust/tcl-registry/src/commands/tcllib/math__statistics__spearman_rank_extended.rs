//! `math::statistics::spearman-rank-extended` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::spearman-rank-extended",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Extended Spearman rank correlation.",
            &["math::statistics::spearman-rank-extended sample_a sample_b"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
