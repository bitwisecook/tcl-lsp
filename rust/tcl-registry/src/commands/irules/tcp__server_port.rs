//! `TCP::server_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::server_port",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the remote TCP port/service number of the serverside TCP connection.",
            synopsis: &["TCP::server_port"],
            snippet: "Returns the remote TCP port/service number of the serverside TCP\nconnection. This command is equivalent to the TCP::remote_port command\nin a serverside context, and to the BIG-IP 4.x variable server_port.",
            source: "https://clouddocs.f5.com/api/irules/TCP__server_port.html",
            examples: "when SERVER_CONNECTED {\n   # This logs information about:\n   #  * the clientside part of the client<->LTM connection, and\n   #  * the serverside part of the LTM<->server connection.\nlog local0.info \"Complete connection: [IP::client_addr]:[TCP::client_port]<->LTM<->[IP::server_addr]:[TCP::server_port]\"\n}",
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
            FormSpec { kind: FormKind::Default, synopsis: "TCP::server_port" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Server,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
