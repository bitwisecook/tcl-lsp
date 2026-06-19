//! `TCP::dsack` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::dsack",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Toggles TCP duplicate selective acknowledgments (D-SACK).",
            synopsis: &["TCP::dsack BOOL_VALUE"],
            snippet: "Enables or disables TCP duplicate selective acknowledgements.\nWhen enabled, accepts D-SACKs from remote hosts, which explicitly acknowledge duplicate packets and allow more accurate reaction to out-of-order and late packets.  See RFC3708 for details.",
            source: "https://clouddocs.f5.com/api/irules/TCP__dsack.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port].\"\n    # Set client-side D-SACKs to enabled.\n    clientside {\n        TCP::dsack enable\n    }\n    # Set server-side D-SACKs to disabled.\n    serverside {\n        TCP::dsack disable\n    }\n}",
            return_value: "None.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::dsack BOOL_VALUE",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
