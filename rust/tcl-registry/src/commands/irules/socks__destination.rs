//! `SOCKS::destination` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SOCKS::destination",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command allows you to get or set the SOCKS destination host or port.",
            synopsis: &["SOCKS::destination ('host')? (HOST_ADDRESS)?", "SOCKS::destination 'port' (PORT)?"],
            snippet: "This command allows you to get or set the SOCKS host or port, individually, or both at the same time.\n\nDetails (Syntax):\nSOCKS::destination \"hostname:port\"\n    Sets the destination to the given hostname and port tuple.\n\nSOCKS::destination\n    Gets the destination in the format \"hostname:port\".\n\nSOCKS::destination host \"hostname\"\n    Sets the destination to the given hostname, doesn't change the port.\nSOCKS::destination host\n    Gets the destination hostname.  (Without appending the port.)\n\nSOCKS::destination port \"port_number\"\n    Sets the destination port, doesn't change the hostname.",
            source: "https://clouddocs.f5.com/api/irules/SOCKS__destination.html",
            examples: "when SOCKS_REQUEST {\n    SOCKS::destination example.com:1234\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SOCKS"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SOCKS::destination ('host')? (HOST_ADDRESS)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
