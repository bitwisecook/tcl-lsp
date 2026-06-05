//! `IP::remote_addr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::remote_addr",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the IP address of the host on the far end of the connection.",
            synopsis: &["IP::remote_addr (clientside | serverside)?"],
            snippet: "Returns the IP address of the host on the far end of the connection. In the clientside context, this is the client IP address. In the serverside context this is the node IP address. You can also specify the IP::client_addr and IP::server_addr commands, respectively.\n\nIn BIG-IP 10.x with route domains enabled this command returns the remote IP address in the x.x.x.x%rd of the server or client (depending on the context) that is in any non-default route domain else it returns just the IP address as expected.\n\nThis command is equivalent to the BIG-IP 4.X variable remote_addr.",
            source: "https://clouddocs.f5.com/api/irules/IP__remote_addr.html",
            examples: "when SERVER_CONNECTED {\n   log local0. \"Node IP address is: [IP::remote_addr]\"\n}",
            return_value: "IP address of the host on the far end of the connection",
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
