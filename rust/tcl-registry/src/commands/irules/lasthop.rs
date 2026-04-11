//! `lasthop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lasthop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the lasthop of an IP connection.",
            &["lasthop (VLAN_OBJ)? (IP_ADDR | MAC_ADDR)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
