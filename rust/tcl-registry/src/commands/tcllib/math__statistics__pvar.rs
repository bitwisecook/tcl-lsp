//! `math::statistics::pvar` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::pvar",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the population variance.",
            &["math::statistics::pvar values"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
