//! `UDP::max_buf_pkts` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::max_buf_pkts",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command can be used to set/get the maximum buffer packets value of a UDP co",
            &["UDP::max_buf_pkts (UDP_MAX_BUF_PKTS)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
