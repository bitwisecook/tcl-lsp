//! `math::statistics::control-xbar` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::control-xbar",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Compute X-bar control chart limits.",
            &["math::statistics::control-xbar data ?nsamples?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
