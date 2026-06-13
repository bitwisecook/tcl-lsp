//! `TCP::unused_port` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::unused_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns an unused TCP port for the specified IP tuple.",
            synopsis: &["TCP::unused_port REMOTE_ADDR REMOTE_PORT LOCAL_ADDR"],
            snippet: "Returns an unused TCP port for the specified IP tuple, using the value\nof <hint_port> as a starting point if it is supplied. If no appropriate\nunused local port could be found, 0 is returned.",
            source: "https://clouddocs.f5.com/api/irules/TCP__unused_port.html",
            examples: "when HTTP_REQUEST {\n    log local0. [TCP::unused_port 192.168.1.124 80 192.168.1.146]\n    }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &[],
            also_in: &["SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::unused_port REMOTE_ADDR REMOTE_PORT LOCAL_ADDR" },
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
