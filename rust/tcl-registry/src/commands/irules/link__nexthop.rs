//! `LINK::nexthop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LINK::nexthop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the MAC address of the next hop.",
            &["LINK::nexthop ('id' | 'type' | 'name')?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
