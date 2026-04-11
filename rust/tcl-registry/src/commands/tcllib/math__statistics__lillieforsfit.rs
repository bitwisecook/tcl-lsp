//! `math::statistics::lillieforsFit` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::lillieforsFit",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Lilliefors goodness-of-fit test.",
            &["math::statistics::lillieforsFit values"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
