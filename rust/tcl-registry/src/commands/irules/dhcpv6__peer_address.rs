//! `DHCPv6::peer_address` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv6::peer_address",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns peer address field from DHCPv6 RELAY message.",
            &["DHCPv6::peer_address"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
