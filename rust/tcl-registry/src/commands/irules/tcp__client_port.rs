//! `TCP::client_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::client_port",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the client port of the TCP connection.",
            synopsis: &["TCP::client_port"],
            snippet: "Returns the TCP port/service number of the clientside TCP\nconnection. This command is equivalent to the TCP::remote_port\ncommand in a clientside context, and to the BIG-IP 4.x variable\nclient_port.",
            source: "https://clouddocs.f5.com/api/irules/TCP__client_port.html",
            examples: "when SERVER_CONNECTED {\n   # This logs information about:\n   #  * the clientside part of the client<->LTM connection, and\n   #  * the serverside part of the LTM<->server connection.\n   log local0.info \"Complete connection: [IP::client_addr]:[TCP::client_port]<->LTM<->[IP::server_addr]:[TCP::server_port]\"\n}",
            return_value: "The port advertised by the client. Even on SERVER events, it still returns the client port from the clientside.",
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
            FormSpec { kind: FormKind::Default, synopsis: "TCP::client_port" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Client,
            },
        ],
        taint_source: Some(TaintColour::TAINTED.union(TaintColour::PORT)),
        ..CommandSpec::DEFAULT
    }
}
