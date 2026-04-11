//! `DHCPv4::htype` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::htype",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns htype (hardware type) field from DHCPv4 message.",
            &["DHCPv4::htype"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
