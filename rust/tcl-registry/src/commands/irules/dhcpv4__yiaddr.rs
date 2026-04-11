//! `DHCPv4::yiaddr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::yiaddr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns yiaddr(your IP) field from DHCPv4 message.",
            &["DHCPv4::yiaddr"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
