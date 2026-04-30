//! `TCP::sendbuf` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::sendbuf",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command can be used to set/get the send buffer size of a TCP connection.",
            &["TCP::sendbuf ('auto' | BUFFER_SIZE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
