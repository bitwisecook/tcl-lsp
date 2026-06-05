//! `TCP::option` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::option",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(2, 4),
        hover: Some(HoverSnippet::brief(
            "Retrieves or changes TCP header options.",
            &["TCP::option get <kind>"],
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
