//! `TCP::remote_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::remote_port",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the remote TCP port number of a connection.",
            synopsis: &["TCP::remote_port (clientside | serverside)?"],
            snippet: "Returns the remote TCP port/service number of a TCP connection. This\ncommand is equivalent to the BIG-IP 4.X variable remote_port. When used\nin a clientside context, this command returns the client-side TCP\nsource port, and is equivalent to the TCP::client_port command.\nWhen used in a serverside context, this command returns the server-side\nTCP destination port, and is equivalent to the TCP::server_port\ncommand.",
            source: "https://clouddocs.f5.com/api/irules/TCP__remote_port.html",
            examples: "when SERVER_CONNECTED {\n  if {[TCP::remote_port] != 443} {\n    SSL::disable\n  }\n}",
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
            FormSpec { kind: FormKind::Default, synopsis: "TCP::remote_port (clientside | serverside)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        taint_source: Some(TaintColour::TAINTED.union(TaintColour::PORT)),
        ..CommandSpec::DEFAULT
    }
}
