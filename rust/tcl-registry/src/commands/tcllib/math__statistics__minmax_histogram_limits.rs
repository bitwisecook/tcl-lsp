//! `math::statistics::minmax-histogram-limits` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::minmax-histogram-limits",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet::brief(
            "Compute histogram limits from min and max.",
            &["math::statistics::minmax-histogram-limits min max ?number?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
