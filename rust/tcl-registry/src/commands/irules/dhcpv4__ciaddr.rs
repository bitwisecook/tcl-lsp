//! `DHCPv4::ciaddr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::ciaddr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns ciaddr (client ip address) from DHCPv4 message.",
            &["DHCPv4::ciaddr"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
