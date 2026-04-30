//! `SDP::session_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SDP::session_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get the SDP session id.",
            &["SDP::session_id"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
