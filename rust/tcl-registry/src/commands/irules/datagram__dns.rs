//! `DATAGRAM::dns` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DATAGRAM::dns",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns DNS header information.",
            synopsis: &["DATAGRAM::dns (id | qr | opcode | qdcount | ancount | nscount | arcount)"],
            snippet: "This iRules command returns DNS header information.\n  Note: L4 protocol of the packet must be either TCP or UDP for this command\n        to work. Also, the L4 port must be equal to the dns port (typically port 53).\n\nDATAGRAM::dns id\n\n     * Returns DNS header 16-bit ‘identification’ field as an integer value.\n\nDATAGRAM::dns qr\n\n     * Returns DNS header ‘query/response’ as a boolean value.\n       0 indicates a ‘query’ and 1 indicates a ‘response’.\n\nDATAGRAM::dns opcode\n\n     * Returns DNS header ‘opcode’ as a string.",
            source: "https://clouddocs.f5.com/api/irules/DATAGRAM__dns.html",
            examples: "when FLOW_INIT {\n  if { [IP::protocol] == 6 } {\n    log local0. \"TCP Payload Length = [DATAGRAM::tcp payload_length] Payload: [DATAGRAM::tcp payload 100]\"\n    log local0. \"DNS Header fields ID: [DATAGRAM::dns id] QR: [DATAGRAM::dns qr] OPCODE: [DATAGRAM::dns opcode] QDCOUNT: [DATAGRAM::dns qdcount]\"\n  }\n  if { [IP::protocol] == 17 } {\n    log local0. \"UDP Payload Length = [DATAGRAM::udp payload_length] Payload: [DATAGRAM::udp payload 100]\"",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FLOW"],
            also_in: &["CLIENT_DATA"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DATAGRAM::dns (id | qr | opcode | qdcount | ancount | nscount | arcount)" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
