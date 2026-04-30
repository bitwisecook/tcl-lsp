//! `MR::ignore_peer_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::ignore_peer_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets or sets the ignore_peer_port mode for the current connection.",
            &["MR::ignore_peer_port (BOOLEAN)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
