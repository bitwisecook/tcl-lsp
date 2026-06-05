//! `relate_client` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "relate_client",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets up a related established connection.",
            synopsis: &["relate_client CONFIG"],
            snippet: "Sets up a related established connection. This can be used with protocols that parse information out of a control connection and then establish a data connection based on information that was exchanged in the control connection.",
            source: "https://clouddocs.f5.com/api/irules/relate_client.html",
            examples: "when SIP_REQUEST {\n    # Taken from https://devcentral.f5.com/wiki/irules.Load-Balance-Outbound-SIP-Voice-Traffic-Signaling-AND-Media-with-SNAT.ashx\n    # Pre-establish the UDP connection to allow RTP from Server -> Client (and vice versa)\n    relate_client {\n        proto 17\n        clientflow $source_VLAN $destination_RTP $destination_RTP_port $source_inside $source_RTP_port\n        serverflow $destination_VLAN $source_outside $source_RTP_port $destination_RTP $destination_RTP_port\n    }\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
