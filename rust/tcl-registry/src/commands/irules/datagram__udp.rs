//! `DATAGRAM::udp` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DATAGRAM::udp",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns UDP payload information.",
            &["DATAGRAM::udp payload (LENGTH)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
