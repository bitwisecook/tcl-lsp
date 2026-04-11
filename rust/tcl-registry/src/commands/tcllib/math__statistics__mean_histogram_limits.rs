//! `math::statistics::mean-histogram-limits` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::mean-histogram-limits",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet::brief(
            "Compute histogram limits from mean and stdev.",
            &["math::statistics::mean-histogram-limits mean stdev ?number?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
