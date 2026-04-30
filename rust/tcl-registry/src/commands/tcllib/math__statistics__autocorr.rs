//! `math::statistics::autocorr` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::autocorr",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the autocorrelation.",
            &["math::statistics::autocorr data"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
