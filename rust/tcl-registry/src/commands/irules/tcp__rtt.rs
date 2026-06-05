//! `TCP::rtt` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::rtt",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the smoothed round-trip time estimate for a TCP connection.",
            synopsis: &["TCP::rtt"],
            snippet: "Returns the smoothed round-trip time estimate for a TCP connection.",
            source: "https://clouddocs.f5.com/api/irules/TCP__rtt.html",
            examples: "when HTTP_RESPONSE {\nclientside { set rtt [TCP::rtt] }\nif {$rtt < 1600 } {\n      log \"NOcompress rtt=$rtt\"\n      COMPRESS::disable\n   }\nelse {\n      log \"compress rtt=$rtt\"\n      COMPRESS::enable\n      COMPRESS::gzip level 9\n   }\n}",
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
            FormSpec { kind: FormKind::Default, synopsis: "TCP::rtt" },
        ],
        ..CommandSpec::DEFAULT
    }
}
