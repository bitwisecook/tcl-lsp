//! `lasthop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lasthop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets the lasthop of an IP connection.",
            synopsis: &["lasthop (VLAN_OBJ)? (IP_ADDR | MAC_ADDR)"],
            snippet: "Sets the lasthop of a IP connection. The lasthop is the MAC destination\nfor packets going back to the client. This is usually the router\n(gateway) that forwards the client's packets to the BIG-IP (if \"auto\nlasthop\" is set), or is determined by the IP routing table. This\ncommand lets you specify the lasthop to use for a particular\nconnection.",
            source: "https://clouddocs.f5.com/api/irules/lasthop.html",
            examples: "when CLIENT_ACCEPTED {\n  lasthop external 01:23:45:ab:cd:ef\n}",
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
            FormSpec { kind: FormKind::Default, synopsis: "lasthop (VLAN_OBJ)? (IP_ADDR | MAC_ADDR)" },
        ],
        ..CommandSpec::DEFAULT
    }
}
