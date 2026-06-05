//! `SIP::header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or sets SIP header information.",
            synopsis: &["SIP::header SIP_HEADER_NAME (INDEX)?", "SIP::header ('value' | 'remove') HEADER_NAME (INDEX)?", "SIP::header ('insert') HEADER_NAME HEADER_VALUE (INDEX)?", "SIP::header 'names'"],
            snippet: "This set of commands allows you to get or set information in the SIP\nheader.\n\nNote: These commands still work on MBLB (Message Based Load\nBalancing) SIP post 11.6+, but there are new commands that only\nrun on MRF (Message Routing Framework) SIP and were introduced\nin 11.6.",
            source: "https://clouddocs.f5.com/api/irules/SIP__header.html",
            examples: "when SIP_REQUEST_SEND {\n  log local0. [SIP::method]\n  SIP::header insert Via [format \"SIP/2.0/TCP %s:%s\" [IP::local_addr] [TCP::local_port]]\n  SIP::header insert Y-Header \"it is yyy\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SIP"],
            also_in: &["MR_EGRESS", "MR_FAILED", "MR_INGRESS"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
