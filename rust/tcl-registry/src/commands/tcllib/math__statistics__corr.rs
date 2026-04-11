//! `math::statistics::corr` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::corr",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Return the correlation coefficient.",
            &["math::statistics::corr data1 data2"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
