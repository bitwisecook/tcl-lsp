//! `UDP::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the content or length of the current UDP payload.",
            synopsis: &["UDP::payload (LENGTH | (OFFSET LENGTH))?", "UDP::payload length", "UDP::payload replace OFFSET LENGTH UDP_PAYLOAD"],
            snippet: "Returns the content or length of the current UDP payload.\nNotice that, unlike TCP, there is no need to trigger a collect, and there is no\ncorresponding release. Moreover, this command is valid not only in\nCLIENT_DATA and SERVER_DATA, but may be invoked within\nCLIENT_ACCEPTED. In that case, it will evaluate to the data\ncontained in the segment that triggered the CLIENT_ACCEPTED event\n- though not necessarily every segment in a UDP stream (see\nCLIENT_ACCEPTED event description for more details).",
            source: "https://clouddocs.f5.com/api/irules/UDP__payload.html",
            examples: "when CLIENT_DATA {\n  # empty payload entirely so there is no packet to send to the server\n  UDP::payload replace 0 [UDP::payload length] \"\"\n\n  # craft a string to hold our  packet data, 0x01 0x00 0x00 0x00 0x02 0x00 0x000x00 0x03 0x00 0x00 0x00\n  set packetdata [binary format i1i1i1 1 2 3 ]\n\n  # then fill payload with our own data from arbitrary length string called packetdata to send to the server",
            return_value: "",
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
        ..CommandSpec::DEFAULT
    }
}
