//! `math::statistics::histogram` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::histogram",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet::brief(
            "Compute a histogram.",
            &["math::statistics::histogram limits values ?weights?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
