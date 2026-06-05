//! `TCP::nagle` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::nagle",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Toggles the Nagle mode.",
            synopsis: &["TCP::nagle (enable | disable | auto)"],
            snippet: "Enables or disables the Nagle algorithm on the current TCP connection.\nNagle waits for additional data before sending undersized packets, see RFC896 for details.\nThe auto option enables or disables Nagle based on connection conditions.",
            source: "https://clouddocs.f5.com/api/irules/TCP__nagle.html",
            examples: "# Change the TCP Nagle mode to auto.\nwhen CLIENT_ACCEPTED {\n    TCP::nagle auto\n}",
            return_value: "None.",
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
