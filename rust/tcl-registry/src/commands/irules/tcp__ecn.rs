//! `TCP::ecn` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::ecn",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Toggles TCP Explicit Congestion Notification.",
            synopsis: &["TCP::ecn BOOL_VALUE"],
            snippet: "Enables or disables TCP explicit congestion notification.\nWhen enabled, respond to explicit router notification of congestion by invoking the TCP congestion response.\nSee RFC3168 for details.",
            source: "https://clouddocs.f5.com/api/irules/TCP__ecn.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port].\"\n    TCP::ecn disable\n}",
            return_value: "None.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::ecn BOOL_VALUE" },
        ],
        ..CommandSpec::DEFAULT
    }
}
