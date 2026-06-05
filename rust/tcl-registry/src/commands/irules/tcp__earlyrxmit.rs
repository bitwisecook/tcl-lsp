//! `TCP::earlyrxmit` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::earlyrxmit",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Toggles TCP early retransmit.",
            synopsis: &["TCP::earlyrxmit (BOOL_VALUE)?"],
            snippet: "Early retransmit allows TCP to assume a packet is lost after fewer than the standard number of duplicate ACKs, if there is no way to send new data and generate more duplicate ACKs (specified in RFC 5827).",
            source: "https://clouddocs.f5.com/api/irules/TCP__earlyrxmit.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port].\"\n    # Set client-side early retransmit to enabled.\n    clientside {\n        log local0. \"Client: earlyrxmit [TCP::earlyrxmit], enabling\"\n        TCP::earlyrxmit enable\n    }\n    # Set server-side early retransmit to disabled.\n    serverside {\n        log local0. \"Server: earlyrxmit [TCP::earlyrxmit], disabling\"\n        TCP::earlyrxmit disable\n    }\n}",
            return_value: "TCP::earlyrxmit returns whether TCP early retransmit is enabled.",
        }),
        ..CommandSpec::DEFAULT
    }
}
