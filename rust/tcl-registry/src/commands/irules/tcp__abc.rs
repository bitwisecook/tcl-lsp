//! `TCP::abc` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::abc",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Toggles Appropriate Byte Counting.",
            synopsis: &["TCP::abc BOOL_VALUE"],
            snippet: "This command will enable or disable TCP Appropriate Byte Counting. Increases congestion window in accordance with bytes actually acknowledged, rather than allowing small acknowledgements to increase the window by an entire segment.",
            source: "https://clouddocs.f5.com/api/irules/TCP__abc.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port].\"\n    # If an HTTP connection, enable ABC on the client side and\n    # disable ABC on the server side.\n    if { [server_port] == 80 } {\n        clientside {\n            TCP::abc enable\n            log local0. \"Client MSS: [TCP::mss]\"\n        }\n        serverside {\n            TCP::abc disable\n            log local0. \"Server MSS: [TCP::mss]\"\n        }\n    }\n}",
            return_value: "None.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::abc BOOL_VALUE",
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
