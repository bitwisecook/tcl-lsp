//! `UDP::server_port` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::server_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the UDP port/service number of a server system.",
            synopsis: &["UDP::server_port"],
            snippet: "Returns the UDP port/service number of the server. This command is\nequivalent to the command serverside { UDP::remote_port }.",
            source: "https://clouddocs.f5.com/api/irules/UDP__server_port.html",
            examples: "when SERVER_CONNECTED {\n    set client [IP::client_addr]:[UDP::client_port]\n    set node [IP::server_addr]:[UDP::server_port]\n    log local0. \"client: $client, server: $server\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("udp"),
            profiles: &[],
            also_in: &[
                "SIP_REQUEST",
                "SIP_REQUEST_SEND",
                "SIP_RESPONSE",
                "STREAM_MATCHED",
            ],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "UDP::server_port" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::UdpState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Server,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
