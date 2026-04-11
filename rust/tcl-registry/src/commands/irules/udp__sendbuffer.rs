//! `UDP::sendbuffer` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::sendbuffer",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command can be used to set/get the maximum send buffer size (bytes) of a UD",
            &["UDP::sendbuffer (UDP_SNDBUF_SIZE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
