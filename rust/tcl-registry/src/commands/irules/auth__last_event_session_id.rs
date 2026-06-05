//! `AUTH::last_event_session_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::last_event_session_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the session ID of the last auth event.",
            synopsis: &["AUTH::last_event_session_id"],
            snippet: "This command returns the session ID of the last auth event, which can\nthen be used to relate to the user behind each session.\n\nAUTH::last_event_session_id\n\n     * Returns the session ID of the last auth event",
            source: "https://clouddocs.f5.com/api/irules/AUTH__last_event_session_id.html",
            examples: "when AUTH_SUCCESS {\n  if {$auth_id eq [AUTH::last_event_session_id]} {\n    log local0. \"auth success event\"\n    set authorized 1\n  }\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
