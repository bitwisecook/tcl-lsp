//! `DATAGRAM::tcp` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DATAGRAM::tcp",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns TCP header and payload information.",
            &["DATAGRAM::tcp (flags | payload_length | window)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
