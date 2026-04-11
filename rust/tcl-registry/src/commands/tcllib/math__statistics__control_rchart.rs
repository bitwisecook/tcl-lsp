//! `math::statistics::control-Rchart` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::control-Rchart",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Compute R-chart control limits.",
            &["math::statistics::control-Rchart data ?nsamples?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
