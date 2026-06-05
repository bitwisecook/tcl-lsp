//! `TCP::notify` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::notify",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Sends a message to upper layers of iRule processing.",
            &["TCP::notify (request | response | eom)"],
            "F5 iRules",
        )),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &[],
            also_in: &["SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
