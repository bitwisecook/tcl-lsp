//! `TAP::insight` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TAP::insight",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Accumulates or sends key:value pairs to TAP, returns token.",
            &["TAP::insight set (TAP_INSIGHT_KEY TAP_INSIGHT_VALUE)*"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
