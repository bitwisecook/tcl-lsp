//! `math::statistics::t-test-mean` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::t-test-mean",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(4),
        hover: Some(HoverSnippet::brief(
            "Perform a t-test on the mean.",
            &["math::statistics::t-test-mean data est_mean est_stdev alpha"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
