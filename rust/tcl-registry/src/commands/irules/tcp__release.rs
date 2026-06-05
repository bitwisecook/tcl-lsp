//! `TCP::release` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::release",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Release data gathered by TCP::collect to the upper layer.",
            synopsis: &["TCP::release (LENGTH)?"],
            snippet: "Causes TCP to release and flush collected data, and allow other\nprotocol layers to resume processing the connection.\n\nReturns the number of bytes actually released. If specified, up to length bytes are released; the return value will tell you how many bytes actually were.",
            source: "https://clouddocs.f5.com/api/irules/TCP__release.html",
            examples: "when CLIENT_ACCEPTED {\n  TCP::collect 15\n}",
            return_value: "The number of bytes released.",
        }),
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
