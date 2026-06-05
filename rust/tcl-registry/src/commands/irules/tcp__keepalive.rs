//! `TCP::keepalive` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::keepalive",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Set/Get the TCP Keep-Alive Interval.",
            synopsis: &["TCP::keepalive (KEEP_ALIVE_INTERVAL)?"],
            snippet: "Sets or gets the number of seconds before BIG-IP sends a keep-alive packet on a TCP connection with no traffic. A value of zero indicates no keep-alive packet should be sent.",
            source: "https://clouddocs.f5.com/api/irules/TCP__keepalive.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port].\"\n    # Set server-side keep-alive interval to 60 seconds.\n    TCP::keepalive 60\n}",
            return_value: "TCP::keepalive without an argument returns the keep-alive interval value of a TCP connection.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::keepalive (KEEP_ALIVE_INTERVAL)?" },
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
