//! `DATAGRAM::dns` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DATAGRAM::dns",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns DNS header information.",
            &["DATAGRAM::dns (id | qr | opcode | qdcount | ancount | nscount | arcount)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
