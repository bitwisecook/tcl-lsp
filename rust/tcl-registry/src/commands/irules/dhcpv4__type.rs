//! `DHCPv4::type` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns type of DHCPv4 message.",
            &["DHCPv4::type"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
