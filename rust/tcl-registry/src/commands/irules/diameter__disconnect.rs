//! `DIAMETER::disconnect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::disconnect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sends Disconnect-Peer-Request to client or server based on context.",
            &["DIAMETER::disconnect ORIGIN_HOST ORIGIN_REALM DIAMETER_DISCONNECT_CAUSE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
