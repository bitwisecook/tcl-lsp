//! `DHCPv4::len` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::len",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns the length of the DHCP packet length.",
            &["DHCPv4::len"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
