//! `math::statistics::pstdev` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::pstdev",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the population standard deviation.",
            &["math::statistics::pstdev values"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
