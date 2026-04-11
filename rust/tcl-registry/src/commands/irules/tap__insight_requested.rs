//! `TAP::insight_requested` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TAP::insight_requested",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "TAP requests insight flag.",
            &["TAP::insight_requested"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
