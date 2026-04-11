//! `math::statistics::test-Wilcoxon` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::test-Wilcoxon",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Wilcoxon signed-rank test.",
            &["math::statistics::test-Wilcoxon sample_a sample_b"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
