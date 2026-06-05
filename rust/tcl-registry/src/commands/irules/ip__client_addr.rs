//! `IP::client_addr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::client_addr",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the client IP address of a connection.",
            synopsis: &["IP::client_addr"],
            snippet: "Returns the client IP address of a connection. This command is equivalent to the command clientside { IP::remote_addr } and to the BIG-IP 4.X variable client_addr.",
            source: "https://clouddocs.f5.com/api/irules/IP__client_addr.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [IP::addr [IP::client_addr] equals 10.10.10.10] } {\n     pool my_pool\n }\n}",
            return_value: "In BIG-IP 10.x with route domains enabled if the client is in any non-default route domain, this command returns the client IP address in the x.x.x.x%rd. For clients in the default route domain, it returns just the IPv4 address.",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["IP_GTM"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
