//! `TCP::idletime` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::idletime",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets the TCP Idle Timeout.",
            synopsis: &["TCP::idletime IDLE_TIME"],
            snippet: "Sets the number of seconds before BIG-IP deletes connections with no traffic. A value of zero indicates no time limit.",
            source: "https://clouddocs.f5.com/api/irules/TCP__idletime.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port].\"\n    # Set server-side idletime to 100.\n    TCP::idletime 100\n}",
            return_value: "None.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::idletime IDLE_TIME" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
