//! `nexthop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "nexthop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Sets the nexthop of an IP connection.", &["nexthop ((IP_ADDR) | ((VLAN_OBJ_NOT_IP_ADDR) (IP_ADDR | MAC_ADDR | transparent)?))"], "F5 iRules")),
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
        ..CommandSpec::DEFAULT
    }
}
