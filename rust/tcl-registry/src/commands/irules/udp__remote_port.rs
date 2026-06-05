//! `UDP::remote_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::remote_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the remote UDP port/service number.",
            synopsis: &["UDP::remote_port (clientside | serverside)?"],
            snippet: "Returns the remote UDP port/service number.",
            source: "https://clouddocs.f5.com/api/irules/UDP__remote_port.html",
            examples: "when SIP_REQUEST {\n  SIP::header insert \"Via\" \"[lindex [split [SIP::via 0] \";\"] 0]received=[IP::client_addr]rport=[UDP::remote_port][lindex [split [SIP::via 0] \";\"] 1]\"\n  SIP::header remove \"Via\" 1\n}",
            return_value: "Returns the remote UDP port/service number",
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
            FormSpec { kind: FormKind::Default, synopsis: "UDP::remote_port (clientside | serverside)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::UdpState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
