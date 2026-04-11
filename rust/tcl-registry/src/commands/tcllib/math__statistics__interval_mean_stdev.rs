//! `math::statistics::interval-mean-stdev` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::interval-mean-stdev",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Compute confidence interval for mean and stdev.",
            &["math::statistics::interval-mean-stdev data confidence"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
