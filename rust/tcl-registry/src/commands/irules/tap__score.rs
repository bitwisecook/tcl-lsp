//! `TAP::score` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TAP::score",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns or updates risk score.",
            &["TAP::score (SCORE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
