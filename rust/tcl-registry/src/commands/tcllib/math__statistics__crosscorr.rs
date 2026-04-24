//! `math::statistics::crosscorr` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::crosscorr",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Return the cross-correlation.",
            &["math::statistics::crosscorr data1 data2"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
