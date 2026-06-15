//! `TCP::mss` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::mss",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the Maximum Segment Size (MSS) for a TCP connection.",
            synopsis: &["TCP::mss"],
            snippet: "Returns the initial connection negotiated MSS. It does not deduct bytes for any common TCP options present in data packets are not deducted. In other words, it is the minimum of the MSS options in the SYN and SYN-ACK packets, or the MSS default of 536 bytes if one packet is missing the option.",
            source: "https://clouddocs.f5.com/api/irules/TCP__mss.html",
            examples: "when HTTP_REQUEST {\n  if { [TCP::mss] >= 1280 } {\n    COMPRESS::disable\n  }\n}",
            return_value: "MSS in bytes.",
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
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::mss",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
