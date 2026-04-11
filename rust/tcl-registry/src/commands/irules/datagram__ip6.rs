//! `DATAGRAM::ip6` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DATAGRAM::ip6",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns ipv6 header information.",
            &["DATAGRAM::ip6 hop_limit"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
