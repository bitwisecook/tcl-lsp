//! `math::statistics::test-Rchart` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::test-Rchart",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Test data against R-chart control limits.",
            &["math::statistics::test-Rchart control data"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
