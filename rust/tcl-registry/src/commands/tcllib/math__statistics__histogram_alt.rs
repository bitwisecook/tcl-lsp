//! `math::statistics::histogram-alt` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::histogram-alt",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet::brief(
            "Compute a histogram (alternate boundaries).",
            &["math::statistics::histogram-alt limits values ?weights?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
