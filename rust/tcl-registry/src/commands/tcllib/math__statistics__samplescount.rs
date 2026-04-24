//! `math::statistics::samplescount` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::samplescount",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet::brief(
            "Count samples matching expression.",
            &["math::statistics::samplescount varname list ?expression?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
