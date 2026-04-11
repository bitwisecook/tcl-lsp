//! `DHCPv4::siaddr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::siaddr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns siaddr(server IP) field from DHCPv4 message.",
            &["DHCPv4::siaddr"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
