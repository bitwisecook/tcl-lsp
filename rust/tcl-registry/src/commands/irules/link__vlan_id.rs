//! `LINK::vlan_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LINK::vlan_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the VLAN tag of the packet.",
            &["LINK::vlan_id"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
