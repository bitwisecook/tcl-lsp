//! `DHCPv4::reject` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::reject",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command drops the packet while sending ICMP packet about the drop reason.",
            &["DHCPv4::reject"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
