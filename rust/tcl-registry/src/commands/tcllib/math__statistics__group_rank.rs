//! `math::statistics::group-rank` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::group-rank",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Rank grouped data.",
            &["math::statistics::group-rank args"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
