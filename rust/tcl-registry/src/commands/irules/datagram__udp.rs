//! `DATAGRAM::udp` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DATAGRAM::udp",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns UDP payload information.",
            synopsis: &["DATAGRAM::udp payload (LENGTH)?", "DATAGRAM::udp payload_length"],
            snippet: "This iRules command returns UDP payload information.\nNote: throws an error if L4 protocol of the current connection is not\nUDP\n\nDATAGRAM::udp payload [<size>]\n\n     * Returns the content of the current UDP payload. If <size> is specified and more than <size>\n       bytes are available, only the first <size> bytes of collected data are returned.\n\nDATAGRAM::udp payload_length\n\n     * Returns the length, in bytes, of the current UDP payload.",
            source: "https://clouddocs.f5.com/api/irules/DATAGRAM__udp.html",
            examples: "when FLOW_INIT {\n  if { [IP::protocol] == 17 } {\n     log local0. \"UDP Flow: [IP::client_addr] [UDP::client_port] --> [IP::local_addr] [UDP::local_port]\"\n     log local0. \"UDP Payload Length = [DATAGRAM::udp payload_length] Payload: [DATAGRAM::udp payload 100]\"\n   }\n}",
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
        ..CommandSpec::DEFAULT
    }
}
