//! `IP::server_addr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::server_addr",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the server's IP address.",
            synopsis: &["IP::server_addr"],
            snippet: "Returns the server's (node's) IP address once a serverside connection has been established. This command is equivalent to the command serverside { IP::remote_addr } and to the BIG-IP 4.X variable server_addr. The command returns 0 if the serverside connection has not been made.\n\nIn BIG-IP 10.x with route domains enabled this command returns the server's (node's) address once the serverside connection is established in the x.x.x.x%rd if the server is in any non-default route domains else it returns just the IPv4 address as expected.",
            source: "https://clouddocs.f5.com/api/irules/IP__server_addr.html",
            examples: "when SERVER_CONNECTED {\n   log local0. \"Node IP address: [IP::server_addr]\"\n}",
            return_value: "server's IP address",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: true,
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
