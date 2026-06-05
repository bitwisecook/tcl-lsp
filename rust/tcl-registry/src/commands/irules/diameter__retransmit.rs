//! `DIAMETER::retransmit` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::retransmit",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Triggers the request associated to the current answer message for retransmission.",
            synopsis: &["DIAMETER::retransmit (disabled | busy | unable | retransmit) (NOTE)?"],
            snippet: "This iRule command triggers the request in the retransmission queue\nthat is associated with the current answer message for\nretransmission. This command will fail the current message is a\nrequest or if there is not an associated request message in the\nretransmission queue.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__retransmit.html",
            examples: "when DIAMETER_EGRESS {\n    if { [DIAMETER::is_response] && ![DIAMETER::is_retransmission] } {\n        log local0. \"reason [DIAMETER::retransmission_reason]\"\n        DIAMETER::retransmit\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
