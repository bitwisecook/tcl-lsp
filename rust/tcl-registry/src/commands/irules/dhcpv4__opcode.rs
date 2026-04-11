//! `DHCPv4::opcode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::opcode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns opcode field from DHCPv4 message.",
            &["DHCPv4::opcode"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
