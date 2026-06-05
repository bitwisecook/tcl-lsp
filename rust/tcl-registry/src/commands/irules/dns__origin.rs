//! `DNS::origin` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::origin",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the originator of the DNS message.",
            synopsis: &["DNS::origin"],
            snippet: "Returns the last module to modify the DNS message. Return values:\n\nCLIENT\n\n   This message has just been received by the BigIP from a client's query,\n   and nothing has been processed.\n\nSERVER\n\n   This message has just been received by the BigIP from a server's\n   response to a DNS query, such as On-Box or Off-Box BIND, or another\n   BigIP entirely.\n\nCACHE\n\n   This message is a response from the DNS Cache.\n\nRPZ\n\n   This message is a response from the Response Policy Zone in your BigIP.\n   It was blocked and either NXDOMAIN or a Walled Garden was returned as a\n   response.",
            source: "https://clouddocs.f5.com/api/irules/DNS__origin.html",
            examples: "equests that were not resolved by DNS Express\n            when DNS_RESPONSE {\n                if { [DNS::origin] ne \"DNSX\" } {\n                    DNS::drop\n                }\n            }",
            return_value: "CLIENT SERVER CACHE GTM_BUILD GTM_REWRITE DNSX DNSSEC LAST_ACTION TCL RPZ RATE_LIMITER",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DNS"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
