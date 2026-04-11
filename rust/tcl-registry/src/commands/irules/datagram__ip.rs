//! `DATAGRAM::ip` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DATAGRAM::ip",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns ip header information.",
            &["DATAGRAM::ip (tos | ttl | flags)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
