//! `UDP::client_port` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::client_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the UDP port/service number of a client system.",
            synopsis: &["UDP::client_port"],
            snippet: "Returns the UDP port/service number of the client system. This command\nis equivalent to the command clientside { UDP::remote_port }.",
            source: "https://clouddocs.f5.com/api/irules/UDP__client_port.html",
            examples: "when CLIENT_DATA {\n  if { [UDP::payload 50] contains \"XYZ\" } {\n    pool xyz_servers\n    persist uie \"[IP::client_addr]:[UDP::client_port]\" 300\n  } else {\n    pool web_servers\n  }\n}",
            return_value: "Returns the UDP port/service number of the client system",
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
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "UDP::client_port",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::UdpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Client,
        }],
        ..CommandSpec::DEFAULT
    }
}
