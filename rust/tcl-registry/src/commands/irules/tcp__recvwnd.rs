//! `TCP::recvwnd` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::recvwnd",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command can be used to set/get the receive window size of a TCP connection.",
            &["TCP::recvwnd ('auto' | WINDOW_SIZE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
