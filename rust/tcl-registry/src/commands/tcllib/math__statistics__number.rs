//! `math::statistics::number` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::number",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the count of values.",
            &["math::statistics::number values"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
