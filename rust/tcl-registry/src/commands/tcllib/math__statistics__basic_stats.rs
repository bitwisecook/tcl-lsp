//! `math::statistics::basic-stats` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::basic-stats",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Compute basic statistics (mean, min, max, count, stdev).",
            &["math::statistics::basic-stats data"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
