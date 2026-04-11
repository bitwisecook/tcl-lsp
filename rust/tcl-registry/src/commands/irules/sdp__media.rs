//! `SDP::media` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SDP::media",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set SDP media information.",
            &["SDP::media (count | MEDIA_INDEX)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
