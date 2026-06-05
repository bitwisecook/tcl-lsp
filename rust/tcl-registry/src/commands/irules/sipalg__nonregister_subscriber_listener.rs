//! `SIPALG::nonregister_subscriber_listener` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIPALG::nonregister_subscriber_listener",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or sets the value of flag enabling creating an ephemeral listener for nonregistered subscribers.",
            synopsis: &["SIPALG::nonregister_subscriber_listener", "SIPALG::nonregister_subscriber_listener (BOOLEAN)"],
            snippet: "Gets or sets the value of flag enabling creating an ephemeral listener for nonregistered subscribers.",
            source: "https://clouddocs.f5.com/api/irules/SIPALG__nonregister_subscriber_listener.html",
            examples: "when SIP_REQUEST {\n    log local0. \"nonregister_subscriber_listener is [SIPALG::nonregister_subscriber_listener]\"\n}",
            return_value: "Returns 1, or 0",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SIP"],
            also_in: &["CLIENT_ACCEPTED", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
