//! `TCP::congestion` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::congestion",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets the TCP congestion control algorithm.",
            synopsis: &["TCP::congestion (none |"],
            snippet: "Changes the TCP congestion control algorithm and initializes any state variables unique to the new algorithm.",
            source: "https://clouddocs.f5.com/api/irules/TCP__congestion.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port].\"\n    # Set client-side congestion control to Woodside.\n    clientside {\n        log local0. \"Client: congestion [TCP::congestion] to woodside\"\n        TCP::congestion woodside\n    }\n    # Set server-side congestion control to High-Speed.\n    serverside {\n        log local0. \"Server: congestion [TCP::congestion] to highspeed\"\n        TCP::congestion highspeed\n    }\n}",
            return_value: "TCP::congestion returns the current congestion control algorithm.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::congestion (none |" },
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
