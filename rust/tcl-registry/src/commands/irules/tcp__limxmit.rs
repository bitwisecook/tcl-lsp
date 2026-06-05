//! `TCP::limxmit` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::limxmit",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Toggles the TCP limited transmit.",
            synopsis: &["TCP::limxmit BOOL_VALUE"],
            snippet: "Enables or disables TCP limited transmit recovery, which sends new data in response to duplicate acks. See RFC3042 for details.",
            source: "https://clouddocs.f5.com/api/irules/TCP__limxmit.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port].\"\n    # Set server-side limited transmit to disable.\n    TCP::limxmit disable\n}",
            return_value: "None.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::limxmit BOOL_VALUE" },
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
