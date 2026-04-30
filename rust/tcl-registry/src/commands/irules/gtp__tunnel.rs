//! `GTP::tunnel` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::tunnel",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "These commands parse the payload of G-PDU as IP datagram and return the values f",
            &["GTP::tunnel ('is_ip'"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
