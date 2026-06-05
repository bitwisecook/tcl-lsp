//! `IP::local_addr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::local_addr",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the IP address of the virtual server the client is connected to or the self-ip LTM is connected from.",
            synopsis: &["IP::local_addr (clientside | serverside)?"],
            snippet: "When called in a clientside context, this command returns the IP address of the virtual server the client is connected to. When called in a serverside context it returns the self-ip address or spoofed client IP address LTM is using for the serverside connection.\n\nThis command is primarily useful for generic rules that are re-used. Also, it is useful in reusing the connected endpoint in another statement (such as with the listen command) or to make routing type decisions. You can also specify the IP::client_addr and IP::server_addr commands.\n\nThis command in BIG-IP 10.",
            source: "https://clouddocs.f5.com/api/irules/IP__local_addr.html",
            examples: "when SERVER_CONNECTED {\n   log local0. \"Source IP address for connection to node: [IP::local_addr]\"\n}",
            return_value: "Returns the IP address being used in the connection.",
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
