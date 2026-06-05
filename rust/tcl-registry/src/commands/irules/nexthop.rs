//! `nexthop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "nexthop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets the nexthop of an IP connection.",
            synopsis: &["nexthop ((IP_ADDR) | ((VLAN_OBJ_NOT_IP_ADDR) (IP_ADDR | MAC_ADDR | transparent)?))"],
            snippet: "Sets the nexthop of an IP connection. The nexthop is the destination\nfor packets going from the BIG-IP to the server. This is usually\ndetermined by the IP routing table. This command lets you specify the\nnexthop to use for a particular connection.\n\nNote: In 11.6, you can use the 'nexthop' command to direct traffic over\n    IPIP tunnels.  In 13.0, you can use the 'nexthop' command to make\n    connections L2 transparent (preserve source and destination MAC address).",
            source: "https://clouddocs.f5.com/api/irules/nexthop.html",
            examples: "when CLIENT_ACCEPTED {\n  nexthop external 01:23:45:ab:cd:ef\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["PERSIST_DOWN"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "nexthop ((IP_ADDR) | ((VLAN_OBJ_NOT_IP_ADDR) (IP_ADDR | MAC_ADDR | transparent)?))" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
