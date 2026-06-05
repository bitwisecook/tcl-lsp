//! `SDP::session_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SDP::session_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get the SDP session id.",
            synopsis: &["SDP::session_id"],
            snippet: "This command allows you to get SDP session id for the current\nconnection",
            source: "https://clouddocs.f5.com/api/irules/SDP__session_id.html",
            examples:
                "when SIP_RESPONSE {\n    log local0. \"SDP SessionID: [SDP::session_id]\"\n}",
            return_value: "Returns the value of the SDP session id.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SDP::session_id",
        }],
        ..CommandSpec::DEFAULT
    }
}
