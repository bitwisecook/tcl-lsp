//! `DHCPv4::chaddr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::chaddr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns chaddr (client hardware address) from DHCPv4 message.",
            &["DHCPv4::chaddr"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
