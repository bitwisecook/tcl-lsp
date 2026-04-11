//! `AUTH::last_event_session_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::last_event_session_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the session ID of the last auth event.",
            &["AUTH::last_event_session_id"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
