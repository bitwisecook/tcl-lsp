//! `DHCPv4::giaddr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::giaddr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns giaddr (gateway or relay IP) field from DHCPv4 message.",
            &["DHCPv4::giaddr"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
