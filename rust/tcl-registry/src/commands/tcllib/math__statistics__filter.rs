//! `math::statistics::filter` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::filter",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(3),
        hover: Some(HoverSnippet::brief(
            "Filter data by expression.",
            &["math::statistics::filter varname data expression"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
