//! `ASM::client_ip` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::client_ip",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the IP address of the end client that sent the request.",
            synopsis: &["ASM::client_ip"],
            snippet: "Returns the IP address of the end client that sent the request.\nNote that this IP address is not necessarily equal to the address\nreturned by the command IP::client_addr, which is the IP address of the\nimmediate client found in the IP header as received by BIG-IP. The\nlatter can be a proxy, in which case the end client IP address is\nextracted from one of the HTTP headers, typically, X-Forwarded-For.",
            source: "https://clouddocs.f5.com/api/irules/ASM__client_ip.html",
            examples: "when ASM_REQUEST_DONE {\n  log local0. \"Src IP: [IP::client_addr], End-client IP: [ASM::client_ip]\"\n}",
            return_value: "Returns the IP address of the end client that sent the request.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ASM"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ASM::client_ip" },
        ],
        ..CommandSpec::DEFAULT
    }
}
