//! `TCP::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 4),
        hover: Some(HoverSnippet::brief(
            "Returns or changes the data collected by TCP::collect.",
            &["TCP::payload ?<size>?"],
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
