//! `TCP::delayed_ack` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::delayed_ack",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Toggles TCP delayed acknowledgements (ACKs).",
            synopsis: &["TCP::delayed_ack BOOL_VALUE"],
            snippet: "Enables or disables TCP delayed acknowledgements.\nWhen enabled, minimizes acknowledgment traffic from BIG-IP by waiting 100ms for additional data to arrive, allowing aggregated ACKs. Can have negative performance implications for some remote hosts depending on their congestion control implementation.",
            source: "https://clouddocs.f5.com/api/irules/TCP__delayed_ack.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port].\"\n    # Set client-side delayed ACKs to enabled.\n    clientside {\n        TCP::delayed_ack enable\n    }\n    # Set server-side delayed ACKs to disabled.\n    serverside {\n        TCP::delayed_ack disable\n    }\n}",
            return_value: "None.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::delayed_ack BOOL_VALUE" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
