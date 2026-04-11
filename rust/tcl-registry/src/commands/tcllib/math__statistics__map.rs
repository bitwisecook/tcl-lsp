//! `math::statistics::map` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::map",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(3),
        hover: Some(HoverSnippet::brief(
            "Map data by expression.",
            &["math::statistics::map varname data expression"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
