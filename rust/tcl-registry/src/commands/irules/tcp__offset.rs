//! `TCP::offset` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::offset",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the number of bytes held in memory via TCP::collect.",
            synopsis: &["TCP::offset"],
            snippet: "Returns the number of bytes currently held in memory via\nTCP::collect. This data is available via TCP::payload.",
            source: "https://clouddocs.f5.com/api/irules/TCP__offset.html",
            examples: "when CLIENT_ACCEPTED {\n  TCP::collect\n}",
            return_value: "The number of bytes collected.",
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
